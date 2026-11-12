//! Mathematical primitives for Moce.
//!
//! Everything in the UI is positioned and sized using these types.
//! Keeping them small and well-defined makes the rest of the codebase
//! predictable and easy to reason about.

pub mod color;
pub mod easing;
pub mod lerp;
pub mod rect;
pub mod vec2;

pub use color::Color;
pub use easing::*;
pub use lerp::Lerp;
pub use rect::{Insets, Rect, Size};
pub use vec2::Vec2;
