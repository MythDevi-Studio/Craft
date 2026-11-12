//! Cursor and selection model.
//!
//! The cursor has a position and an optional selection anchor. When the
//! anchor is set, the text between the anchor and the cursor position is
//! selected. Both are `Position` values (line, column).

use crate::editor::buffer::Position;

/// A cursor with optional selection.
///
/// `anchor` is where the selection starts (set when the user begins dragging
/// or shift-clicking). `position` is the active caret position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Cursor {
    /// The active caret position.
    pub position: Position,
    /// If set, the selection extends from `anchor` to `position`.
    pub anchor: Option<Position>,
}

impl Cursor {
    /// Create a cursor at the given position with no selection.
    pub fn at(position: Position) -> Self {
        Self {
            position,
            anchor: None,
        }
    }

    /// Create a cursor at (line, col) with no selection.
    pub fn at_pos(line: usize, col: usize) -> Self {
        Self::at(Position::new(line, col))
    }

    /// Whether there is an active selection.
    #[inline]
    pub fn has_selection(&self) -> bool {
        self.anchor.is_some_and(|a| a != self.position)
    }

    /// Start a selection at the current position.
    pub fn start_selection(&mut self) {
        self.anchor = Some(self.position);
    }

    /// Clear the selection (collapse to the caret position).
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Select everything from `start` to `end`.
    pub fn set_selection(&mut self, start: Position, end: Position) {
        if start <= end {
            self.anchor = Some(start);
            self.position = end;
        } else {
            self.anchor = Some(end);
            self.position = start;
        }
    }

    /// Select an entire line.
    pub fn select_line(&mut self, line: usize) {
        self.anchor = Some(Position::new(line, 0));
        self.position = Position::new(line, usize::MAX);
    }

    /// Get the selection range as (start, end) positions.
    /// Returns None if there is no selection.
    pub fn selection_range(&self) -> Option<(Position, Position)> {
        self.anchor.map(|a| {
            if a <= self.position {
                (a, self.position)
            } else {
                (self.position, a)
            }
        })
    }

    /// Move the cursor by a delta, clearing the selection unless `extend` is true.
    pub fn move_to(&mut self, position: Position, extend: bool) {
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.position);
            }
        } else {
            self.anchor = None;
        }
        self.position = position;
    }
}

/// A selection range (used for multiple cursors in the future).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
}

impl Selection {
    pub fn new(start: Position, end: Position) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    pub fn contains(&self, pos: Position) -> bool {
        pos >= self.start && pos < self.end
    }
}
