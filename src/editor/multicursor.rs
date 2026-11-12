//! Multiple-cursor support.
//!
//! Moce supports multiple independent cursors (like VS Code's Alt+Click or
//! Ctrl+D to add next occurrence). Each cursor is an independent
//! (position, anchor) pair. When editing, all cursors are applied from
//! last to first (by offset) so that earlier offsets stay valid after
//! later edits.

use crate::editor::buffer::Position;
use crate::editor::cursor::Cursor;

/// A named cursor ID (so the UI can track which cursor is "primary").
pub type CursorId = usize;

/// The multi-cursor set.
///
/// The primary cursor is always at index 0. Additional cursors are appended.
/// Duplicate positions are merged automatically.
pub struct MultiCursor {
    cursors: Vec<Cursor>,
    next_id: CursorId,
}

impl MultiCursor {
    pub fn new() -> Self {
        Self {
            cursors: vec![Cursor::at(Position::ZERO)],
            next_id: 1,
        }
    }

    /// The primary cursor.
    pub fn primary(&self) -> &Cursor {
        &self.cursors[0]
    }

    /// Mutable reference to the primary cursor.
    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[0]
    }

    /// All cursors (including primary).
    pub fn all(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Mutable reference to all cursors.
    pub fn all_mut(&mut self) -> &mut Vec<Cursor> {
        &mut self.cursors
    }

    /// Add a new cursor at the given position. Returns the new cursor's index.
    pub fn add(&mut self, position: Position) -> usize {
        let cur = Cursor::at(position);
        self.cursors.push(cur);
        self.merge_duplicates();
        self.cursors.len() - 1
    }

    /// Remove all secondary cursors, keeping only the primary.
    pub fn reset_to_primary(&mut self) {
        self.cursors.truncate(1);
    }

    /// Whether there are multiple cursors.
    pub fn is_multi(&self) -> bool {
        self.cursors.len() > 1
    }

    /// Move all cursors by the same direction.
    pub fn move_all<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Cursor),
    {
        for cur in &mut self.cursors {
            f(cur);
        }
        self.merge_duplicates();
    }

    /// Merge cursors at the same position (deduplicate).
    fn merge_duplicates(&mut self) {
        self.cursors.dedup_by_key(|c| c.position);
    }

    /// Sort cursors by position (ascending).
    pub fn sort_by_position(&mut self) {
        self.cursors.sort_by_key(|c| c.position);
    }

    /// Sort cursors in reverse (for applying edits last-to-first).
    pub fn sort_by_position_desc(&mut self) {
        self.cursors.sort_by_key(|c| std::cmp::Reverse(c.position));
    }

    /// Move cursor offsets after an edit (insert/delete at a given position).
    /// This keeps all secondary cursors valid after a primary edit.
    pub fn adjust_after_insert(&mut self, insert_line: usize, insert_col: usize, lines_added: usize, cols_added: usize) {
        for (i, cur) in self.cursors.iter_mut().enumerate() {
            if i == 0 { continue; }
            if cur.position.line > insert_line
                || (cur.position.line == insert_line && cur.position.col >= insert_col)
            {
                if lines_added == 0 {
                    cur.position.col += cols_added;
                } else {
                    cur.position.line += lines_added;
                }
                // Adjust anchor too
                if let Some(ref mut a) = cur.anchor {
                    if a.line > insert_line || (a.line == insert_line && a.col >= insert_col) {
                        if lines_added == 0 { a.col += cols_added; }
                        else { a.line += lines_added; }
                    }
                }
            }
        }
    }

    pub fn adjust_after_delete(&mut self, del_line: usize, del_col: usize, lines_removed: usize, cols_removed: usize) {
        for (i, cur) in self.cursors.iter_mut().enumerate() {
            if i == 0 { continue; }
            if cur.position.line > del_line
                || (cur.position.line == del_line && cur.position.col >= del_col + cols_removed)
            {
                if lines_removed == 0 {
                    cur.position.col = cur.position.col.saturating_sub(cols_removed);
                } else {
                    cur.position.line = cur.position.line.saturating_sub(lines_removed);
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.cursors.len()
    }
}

impl Default for MultiCursor {
    fn default() -> Self {
        Self::new()
    }
}

use std::cmp::Reverse;
