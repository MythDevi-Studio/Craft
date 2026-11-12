//! Text buffer backed by a rope data structure.
//!
//! A rope provides O(log n) insertion and deletion at arbitrary positions,
//! which is essential for responsive editing of large files. The `ropey`
//! crate handles the internal tree management; this module wraps it with
//! editor-friendly methods.

use ropey::iter::Chars;
use ropey::Rope;

/// A UTF-8 text buffer optimized for editing.
///
/// Line and column numbers are zero-based internally. The UI layer converts
/// to one-based for display (line numbers start at 1).
#[derive(Clone)]
pub struct Buffer {
    rope: Rope,
}

impl Buffer {
    /// Create an empty buffer.
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
        }
    }

    /// Create a buffer from a string.
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }

    /// Total number of characters in the buffer.
    #[inline]
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Total number of lines (including the final line even if it has no trailing newline).
    #[inline]
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Whether the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Get the character at a linear char offset.
    pub fn char_at(&self, offset: usize) -> Option<char> {
        if offset < self.rope.len_chars() {
            self.rope.get_char(offset)
        } else {
            None
        }
    }

    /// Get the length (in chars) of a specific line.
    pub fn line_len(&self, line: usize) -> usize {
        if line < self.rope.len_lines() {
            let line_slice = self.rope.line(line);
            // Ropey counts the trailing newline as part of the line.
            // Strip it so callers get the actual content length.
            let raw_len = line_slice.len_chars();
            if line < self.rope.len_lines() - 1 {
                // Lines before the last include a trailing newline char.
                raw_len.saturating_sub(1)
            } else {
                raw_len
            }
        } else {
            0
        }
    }

    /// Get the content of a specific line as a String (without trailing newline).
    pub fn line(&self, line: usize) -> String {
        if line < self.rope.len_lines() {
            let line_slice = self.rope.line(line);
            let s = line_slice.to_string();
            // Strip trailing newline if present (all lines except possibly the last).
            if line < self.rope.len_lines() - 1 {
                s.trim_end_matches('\n').to_string()
            } else {
                s
            }
        } else {
            String::new()
        }
    }

    /// Get a slice of the text as a String.
    pub fn slice(&self, start: usize, end: usize) -> String {
        let start = start.min(self.rope.len_chars());
        let end = end.min(self.rope.len_chars()).max(start);
        self.rope
            .get_slice(start..end)
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    /// Get the entire text as a String.
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// Convert a (line, col) position to a linear char offset.
    pub fn position_to_offset(&self, pos: Position) -> usize {
        let line = pos.line.min(self.rope.len_lines().saturating_sub(1));
        let line_start = self.rope.line_to_char(line);
        let col = pos.col.min(self.line_len(line));
        line_start + col
    }

    /// Convert a linear char offset to a (line, col) position.
    pub fn offset_to_position(&self, offset: usize) -> Position {
        let offset = offset.min(self.rope.len_chars());
        let line = self.rope.char_to_line(offset);
        let line_start = self.rope.line_to_char(line);
        let col = offset - line_start;
        Position { line, col }
    }

    /// Insert text at a linear char offset.
    pub fn insert(&mut self, offset: usize, text: &str) {
        let offset = offset.min(self.rope.len_chars());
        self.rope.insert(offset, text);
    }

    /// Insert text at a (line, col) position.
    pub fn insert_at(&mut self, pos: Position, text: &str) {
        let offset = self.position_to_offset(pos);
        self.insert(offset, text);
    }

    /// Remove a range of characters [start, end).
    pub fn remove(&mut self, start: usize, end: usize) {
        let start = start.min(self.rope.len_chars());
        let end = end.min(self.rope.len_chars()).max(start);
        if start < end {
            self.rope.remove(start..end);
        }
    }

    /// Replace a range of characters [start, end) with new text.
    pub fn replace(&mut self, start: usize, end: usize, text: &str) {
        let start = start.min(self.rope.len_chars());
        let end = end.min(self.rope.len_chars()).max(start);
        if start < end {
            self.rope.remove(start..end);
        }
        self.rope.insert(start, text);
    }

    /// Get an iterator over characters.
    pub fn chars(&self) -> Chars<'_> {
        self.rope.chars()
    }

    /// Get the indentation (leading whitespace) of a line.
    pub fn line_indent(&self, line: usize) -> usize {
        let line_text = self.line(line);
        line_text
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .count()
    }

    /// Find the matching word boundary to the left of the given offset.
    /// Returns the offset where the word starts.
    pub fn word_left(&self, offset: usize) -> usize {
        if offset == 0 {
            return 0;
        }
        let mut pos = offset;
        // Skip whitespace going left
        while pos > 0 && self.char_at(pos - 1).map_or(false, |c| c.is_whitespace()) {
            pos -= 1;
        }
        // Skip word characters going left
        while pos > 0 {
            let c = self.char_at(pos - 1);
            match c {
                Some(ch) if ch.is_alphanumeric() || ch == '_' => pos -= 1,
                _ => break,
            }
        }
        pos
    }

    /// Find the matching word boundary to the right of the given offset.
    /// Returns the offset where the word ends.
    pub fn word_right(&self, offset: usize) -> usize {
        let len = self.rope.len_chars();
        if offset >= len {
            return len;
        }
        let mut pos = offset;
        // Skip word characters going right
        while pos < len {
            let c = self.char_at(pos);
            match c {
                Some(ch) if ch.is_alphanumeric() || ch == '_' => pos += 1,
                _ => break,
            }
        }
        // Skip whitespace going right
        while pos < len && self.char_at(pos).map_or(false, |c| c.is_whitespace()) {
            pos += 1;
        }
        pos
    }

    /// Get the underlying rope (read-only access for syntax highlighting, etc.).
    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

/// A position in the text buffer, as (line, column) in character coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Position {
    /// Zero-based line number.
    pub line: usize,
    /// Zero-based column (character offset within the line).
    pub col: usize,
}

impl Position {
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    pub const ZERO: Self = Self::new(0, 0);
}
