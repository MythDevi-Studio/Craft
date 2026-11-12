//! Syntax highlighting engine.
//!
//! A lightweight tokenizer that classifies text into token types for
//! coloring. This is intentionally simple — not a full parser — but
//! handles common cases well enough to make code readable.

pub mod highlighter;
pub mod token;

pub use highlighter::Highlighter;
pub use token::{Token, TokenType};
