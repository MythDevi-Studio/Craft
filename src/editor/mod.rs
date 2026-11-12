//! Editor core: text buffer, cursor, selection, and document management.
//!
//! The editor model is completely independent of the UI layer. It knows
//! nothing about pixels, fonts, or rendering — only about text positions,
//! line numbers, and character offsets. This separation keeps the editing
//! logic testable and reusable.

pub mod buffer;
pub mod cursor;
pub mod document;

pub use buffer::Buffer;
pub use cursor::{Cursor, Position, Selection};
pub use document::Document;
