//! Editor core: text buffer, cursor, selection, and document management.
//!
//! The editor model is completely independent of the UI layer. It knows
//! nothing about pixels, fonts, or rendering — only about text positions,
//! line numbers, and character offsets. This separation keeps the editing
//! logic testable and reusable.

pub mod buffer;
pub mod cursor;
pub mod document;
pub mod history;
pub mod multicursor;
pub mod search;

pub use buffer::{Buffer, Position};
pub use cursor::{Cursor, Selection};
pub use document::{Document, Language};
pub use history::{EditRecord, History, Transaction};
pub use multicursor::MultiCursor;
pub use search::{Match as SearchMatch, SearchOptions, SearchState};
