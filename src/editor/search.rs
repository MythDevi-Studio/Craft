//! Find / replace engine.
//!
//! Searches a document buffer for all occurrences of a pattern (literal or
//! simple regex-like wildcard). Returns a sorted list of `Match` spans that
//! the UI uses to highlight occurrences and jump between them.

use crate::editor::buffer::{Buffer, Position};

/// A single search match: start and end as (line, col) pairs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub start: Position,
    pub end: Position,
}

/// Search options.
#[derive(Clone, Debug, Default)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
}

/// The find/replace state.
pub struct SearchState {
    pub query: String,
    pub replace: String,
    pub options: SearchOptions,
    pub matches: Vec<Match>,
    /// Index of the currently focused match (-1 = none).
    pub current_match: Option<usize>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            replace: String::new(),
            options: SearchOptions::default(),
            matches: Vec::new(),
            current_match: None,
        }
    }

    /// Run the search against `buffer` and populate `self.matches`.
    pub fn update_matches(&mut self, buffer: &Buffer) {
        self.matches.clear();
        self.current_match = None;

        if self.query.is_empty() {
            return;
        }

        let query = if self.options.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        let total_lines = buffer.len_lines();
        for line_idx in 0..total_lines {
            let line_text = buffer.line(line_idx);
            let haystack = if self.options.case_sensitive {
                line_text.clone()
            } else {
                line_text.to_lowercase()
            };

            let mut search_start = 0;
            while let Some(col) = haystack[search_start..].find(&query) {
                let abs_col = search_start + col;
                let char_col = haystack[..abs_col].chars().count();
                let char_len = query.chars().count();

                // Whole-word check
                if self.options.whole_word {
                    let before_ok = abs_col == 0
                        || !haystack[..abs_col]
                            .chars()
                            .last()
                            .map_or(false, |c| c.is_alphanumeric() || c == '_');
                    let after_ok = abs_col + query.len() >= haystack.len()
                        || !haystack[abs_col + query.len()..]
                            .chars()
                            .next()
                            .map_or(false, |c| c.is_alphanumeric() || c == '_');
                    if !before_ok || !after_ok {
                        search_start = abs_col + 1;
                        continue;
                    }
                }

                self.matches.push(Match {
                    start: Position::new(line_idx, char_col),
                    end: Position::new(line_idx, char_col + char_len),
                });

                search_start = abs_col + query.len().max(1);
            }
        }
    }

    /// Move to the next match after `current_pos`, wrapping around.
    pub fn next_match(&mut self, current_pos: Position) -> Option<Match> {
        if self.matches.is_empty() {
            return None;
        }
        // Find first match after current_pos
        let idx = self
            .matches
            .iter()
            .position(|m| m.start > current_pos)
            .unwrap_or(0); // wrap
        self.current_match = Some(idx);
        Some(self.matches[idx])
    }

    /// Move to the previous match before `current_pos`, wrapping around.
    pub fn prev_match(&mut self, current_pos: Position) -> Option<Match> {
        if self.matches.is_empty() {
            return None;
        }
        let idx = self
            .matches
            .iter()
            .rposition(|m| m.start < current_pos)
            .unwrap_or(self.matches.len() - 1); // wrap
        self.current_match = Some(idx);
        Some(self.matches[idx])
    }

    /// Whether a position is inside a match (for highlight rendering).
    pub fn is_match_at(&self, line: usize, col: usize) -> bool {
        let pos = Position::new(line, col);
        self.matches.iter().any(|m| pos >= m.start && pos < m.end)
    }

    /// Whether a (line, col) starts the focused match.
    pub fn is_focused_match_at(&self, line: usize, col: usize) -> bool {
        if let Some(idx) = self.current_match {
            if let Some(m) = self.matches.get(idx) {
                return m.start == Position::new(line, col);
            }
        }
        false
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}
