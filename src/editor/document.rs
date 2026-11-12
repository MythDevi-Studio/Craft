//! Document model: a buffer plus metadata about its file, language, and state.
//!
//! Each open file maps to one `Document`. The document tracks whether it has
//! unsaved changes, what language it is (for syntax highlighting), and the
//! cursor position.

use std::path::PathBuf;

use super::{Buffer, Cursor, Position};

/// Supported programming languages for syntax highlighting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    PlainText,
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Json,
    Markdown,
    Css,
    Html,
}

impl Language {
    /// Guess the language from a file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "ts" | "tsx" => Language::TypeScript,
            "py" => Language::Python,
            "json" => Language::Json,
            "md" | "markdown" => Language::Markdown,
            "css" => Language::Css,
            "html" | "htm" => Language::Html,
            _ => Language::PlainText,
        }
    }

    /// Guess the language from a file path.
    pub fn from_path(path: &std::path::Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(Language::PlainText)
    }

    /// Get the display name of this language.
    pub fn name(self) -> &'static str {
        match self {
            Language::PlainText => "Plain Text",
            Language::Rust => "Rust",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Python => "Python",
            Language::Json => "JSON",
            Language::Markdown => "Markdown",
            Language::Css => "CSS",
            Language::Html => "HTML",
        }
    }

    /// Get the common indentation unit for this language.
    pub fn indent_unit(self) -> &'static str {
        match self {
            Language::Python => "    ",
            _ => "    ", // 4 spaces by default; could be configurable
        }
    }
}

/// A document being edited.
///
/// Contains the text buffer, cursor, file path, language, and dirty flag.
pub struct Document {
    /// The text buffer (rope-backed).
    pub buffer: Buffer,
    /// Primary cursor (multi-cursor can be added later).
    pub cursor: Cursor,
    /// File path if the document was loaded from or saved to a file.
    pub path: Option<PathBuf>,
    /// Programming language for syntax highlighting.
    pub language: Language,
    /// Whether the buffer has unsaved changes.
    pub dirty: bool,
    /// Scroll position (in lines, fractional for smooth scrolling).
    pub scroll_y: f32,
    /// Horizontal scroll position (in pixels).
    pub scroll_x: f32,
}

impl Document {
    /// Create a new empty document.
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            cursor: Cursor::at(Position::ZERO),
            path: None,
            language: Language::PlainText,
            dirty: false,
            scroll_y: 0.0,
            scroll_x: 0.0,
        }
    }

    /// Create a document from text with a guessed or specified language.
    pub fn from_str(text: &str, language: Language) -> Self {
        Self {
            buffer: Buffer::from_str(text),
            cursor: Cursor::at(Position::ZERO),
            path: None,
            language,
            dirty: false,
            scroll_y: 0.0,
            scroll_x: 0.0,
        }
    }

    /// Load a document from a file path.
    pub fn from_file(path: PathBuf) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(&path)?;
        let language = Language::from_path(&path);
        Ok(Self {
            buffer: Buffer::from_str(&text),
            cursor: Cursor::at(Position::ZERO),
            path: Some(path),
            language,
            dirty: false,
            scroll_y: 0.0,
            scroll_x: 0.0,
        })
    }

    /// Insert text at the cursor position and advance the cursor.
    pub fn insert_text(&mut self, text: &str) {
        if let Some((start, end)) = self.cursor.selection_range() {
            // Replace selection
            let offset_start = self.buffer.position_to_offset(start);
            let offset_end = self.buffer.position_to_offset(end);
            self.buffer.replace(offset_start, offset_end, text);
            let new_pos = self.buffer.offset_to_position(offset_start + text.chars().count());
            self.cursor.position = new_pos;
            self.cursor.clear_selection();
        } else {
            let offset = self.buffer.position_to_offset(self.cursor.position);
            self.buffer.insert(offset, text);
            let new_pos = self.buffer.offset_to_position(offset + text.chars().count());
            self.cursor.position = new_pos;
        }
        self.dirty = true;
    }

    /// Insert a newline with auto-indentation based on the current line.
    pub fn insert_newline(&mut self) {
        let line = self.cursor.position.line;
        let current_line = self.buffer.line(line);
        let indent: String = current_line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();

        // Extra indent after opening braces
        let extra = current_line
            .trim_end()
            .ends_with(['{', '[', '(']);
        let new_indent = if extra {
            format!("{}{}", indent, self.language.indent_unit())
        } else {
            indent
        };

        self.insert_text(&format!("\n{}", new_indent));
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if let Some((start, end)) = self.cursor.selection_range() {
            self.delete_range(start, end);
            return;
        }

        let pos = self.cursor.position;
        if pos.col > 0 {
            // Check for smart backspace (remove indent unit)
            let line = self.buffer.line(pos.line);
            let line_indent = self.buffer.line_indent(pos.line);
            let indent_unit = self.language.indent_unit();
            if pos.col == line_indent && pos.col >= indent_unit.len() {
                // Check if the preceding characters form an indent unit
                let before: String = line.chars().take(pos.col).collect();
                if before.ends_with(indent_unit) {
                    self.delete_range(
                        Position::new(pos.line, pos.col - indent_unit.len()),
                        Position::new(pos.line, pos.col),
                    );
                    return;
                }
            }
            // Delete single character
            self.delete_range(Position::new(pos.line, pos.col - 1), pos);
        } else if pos.line > 0 {
            // Join with previous line
            let prev_len = self.buffer.line_len(pos.line - 1);
            self.delete_range(Position::new(pos.line - 1, prev_len), Position::new(pos.line, 0));
        }
    }

    /// Delete the character after the cursor (delete key).
    pub fn delete_forward(&mut self) {
        if let Some((start, end)) = self.cursor.selection_range() {
            self.delete_range(start, end);
            return;
        }

        let pos = self.cursor.position;
        let line_len = self.buffer.line_len(pos.line);
        if pos.col < line_len {
            self.delete_range(pos, Position::new(pos.line, pos.col + 1));
        } else if pos.line < self.buffer.len_lines() - 1 {
            self.delete_range(pos, Position::new(pos.line + 1, 0));
        }
    }

    /// Delete a range of text and move the cursor to the start of the range.
    pub fn delete_range(&mut self, start: Position, end: Position) {
        let offset_start = self.buffer.position_to_offset(start);
        let offset_end = self.buffer.position_to_offset(end);
        self.buffer.remove(offset_start, offset_end);
        self.cursor.position = start;
        self.cursor.clear_selection();
        self.dirty = true;
    }

    /// Delete the word before the cursor.
    pub fn delete_word_back(&mut self) {
        if self.cursor.has_selection() {
            self.backspace();
            return;
        }
        let offset = self.buffer.position_to_offset(self.cursor.position);
        let word_start = self.buffer.word_left(offset);
        let start = self.buffer.offset_to_position(word_start);
        self.delete_range(start, self.cursor.position);
    }

    /// Move the cursor by one character in the given direction.
    pub fn move_cursor(&mut self, direction: CursorDirection, extend: bool) {
        let pos = self.cursor.position;
        let new_pos = match direction {
            CursorDirection::Left => {
                if pos.col > 0 {
                    Position::new(pos.line, pos.col - 1)
                } else if pos.line > 0 {
                    let prev_len = self.buffer.line_len(pos.line - 1);
                    Position::new(pos.line - 1, prev_len)
                } else {
                    pos
                }
            }
            CursorDirection::Right => {
                let line_len = self.buffer.line_len(pos.line);
                if pos.col < line_len {
                    Position::new(pos.line, pos.col + 1)
                } else if pos.line < self.buffer.len_lines() - 1 {
                    Position::new(pos.line + 1, 0)
                } else {
                    pos
                }
            }
            CursorDirection::Up => {
                if pos.line > 0 {
                    let prev_len = self.buffer.line_len(pos.line - 1);
                    Position::new(pos.line - 1, pos.col.min(prev_len))
                } else {
                    Position::new(0, 0)
                }
            }
            CursorDirection::Down => {
                if pos.line < self.buffer.len_lines() - 1 {
                    let next_len = self.buffer.line_len(pos.line + 1);
                    Position::new(pos.line + 1, pos.col.min(next_len))
                } else {
                    let last_line = self.buffer.len_lines() - 1;
                    Position::new(last_line, self.buffer.line_len(last_line))
                }
            }
            CursorDirection::Home => Position::new(pos.line, 0),
            CursorDirection::End => Position::new(pos.line, self.buffer.line_len(pos.line)),
            CursorDirection::DocStart => Position::new(0, 0),
            CursorDirection::DocEnd => {
                let last = self.buffer.len_lines() - 1;
                Position::new(last, self.buffer.line_len(last))
            }
            CursorDirection::WordLeft => {
                let offset = self.buffer.position_to_offset(pos);
                let word_start = self.buffer.word_left(offset);
                self.buffer.offset_to_position(word_start)
            }
            CursorDirection::WordRight => {
                let offset = self.buffer.position_to_offset(pos);
                let word_end = self.buffer.word_right(offset);
                self.buffer.offset_to_position(word_end)
            }
        };
        self.cursor.move_to(new_pos, extend);
    }

    /// Get the selected text, if any.
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.cursor.selection_range()?;
        let offset_start = self.buffer.position_to_offset(start);
        let offset_end = self.buffer.position_to_offset(end);
        Some(self.buffer.slice(offset_start, offset_end))
    }

    /// Select all text in the document.
    pub fn select_all(&mut self) {
        let last_line = self.buffer.len_lines().saturating_sub(1);
        let end = Position::new(last_line, self.buffer.line_len(last_line));
        self.cursor.set_selection(Position::new(0, 0), end);
    }

    /// Save the document to its file path.
    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(path) = &self.path {
            std::fs::write(path, self.buffer.to_string())?;
            self.dirty = false;
        }
        Ok(())
    }

    /// Get the file name for display.
    pub fn display_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(String::from)
            .unwrap_or_else(|| "untitled".to_string())
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// Directions for cursor movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    DocStart,
    DocEnd,
    WordLeft,
    WordRight,
}
