//! Undo/redo history using the command-stack pattern.
//!
//! Each edit produces an `EditRecord` that stores enough information to both
//! apply and reverse the change. Records are grouped into `Transaction`s;
//! a transaction is committed as a single undo unit (e.g. a word deletion,
//! a paste, or a find-replace-all).
//!
//! The history caps at `MAX_HISTORY` transactions to bound memory use.

use crate::editor::buffer::Position;

const MAX_HISTORY: usize = 2048;

/// A single atomic text change (insert or delete).
#[derive(Clone, Debug)]
pub struct EditRecord {
    /// Start offset of the change.
    pub offset: usize,
    /// Text that was inserted (empty on delete).
    pub inserted: String,
    /// Text that was deleted (empty on insert).
    pub deleted: String,
    /// Cursor position after this edit was applied.
    pub cursor_after: Position,
    /// Cursor position before this edit was applied.
    pub cursor_before: Position,
}

impl EditRecord {
    pub fn insert(offset: usize, text: String, before: Position, after: Position) -> Self {
        Self {
            offset,
            inserted: text,
            deleted: String::new(),
            cursor_after: after,
            cursor_before: before,
        }
    }

    pub fn delete(offset: usize, text: String, before: Position, after: Position) -> Self {
        Self {
            offset,
            inserted: String::new(),
            deleted: text,
            cursor_after: after,
            cursor_before: before,
        }
    }

    pub fn replace(
        offset: usize,
        deleted: String,
        inserted: String,
        before: Position,
        after: Position,
    ) -> Self {
        Self {
            offset,
            inserted,
            deleted,
            cursor_after: after,
            cursor_before: before,
        }
    }
}

/// A group of edits committed as one undo step.
#[derive(Clone, Debug, Default)]
pub struct Transaction {
    pub records: Vec<EditRecord>,
}

impl Transaction {
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }

    pub fn push(&mut self, record: EditRecord) {
        self.records.push(record);
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// The undo/redo stack.
pub struct History {
    /// Committed undo stack (index 0 = oldest).
    undo: Vec<Transaction>,
    /// Redo stack (re-populated when user undoes).
    redo: Vec<Transaction>,
    /// Current open transaction (not yet committed).
    current: Option<Transaction>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            current: None,
        }
    }

    /// Begin a new transaction. Any subsequent records are grouped into it.
    pub fn begin(&mut self) {
        if self.current.is_none() {
            self.current = Some(Transaction::new());
        }
    }

    /// Push an edit record into the current transaction.
    /// Auto-begins a transaction if one isn't open.
    pub fn push(&mut self, record: EditRecord) {
        if self.current.is_none() {
            self.begin();
        }
        self.current.as_mut().unwrap().push(record);
        // Any new edit clears the redo stack.
        self.redo.clear();
    }

    /// Commit the current transaction.
    pub fn commit(&mut self) {
        if let Some(tx) = self.current.take() {
            if !tx.is_empty() {
                self.undo.push(tx);
                // Keep the stack bounded
                if self.undo.len() > MAX_HISTORY {
                    self.undo.remove(0);
                }
            }
        }
    }

    /// Commit any pending transaction automatically (e.g. on cursor move).
    pub fn auto_commit(&mut self) {
        self.commit();
    }

    /// Whether there is something to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty() || self.current.as_ref().map_or(false, |t| !t.is_empty())
    }

    /// Whether there is something to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Pop the last committed transaction for undoing.
    pub fn pop_undo(&mut self) -> Option<Transaction> {
        // If there's an open transaction, commit it first so we undo it.
        self.commit();
        self.undo.pop().map(|tx| {
            // Push it onto redo so the user can redo.
            self.redo.push(tx.clone());
            tx
        })
    }

    /// Pop the top of the redo stack.
    pub fn pop_redo(&mut self) -> Option<Transaction> {
        self.redo.pop().map(|tx| {
            self.undo.push(tx.clone());
            tx
        })
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}
