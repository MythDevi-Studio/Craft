//! Custom UI framework built on Skia.
//!
//! This module provides everything needed to build the editor's interface
//! without any external UI toolkit: a theme system, a Skia-backed painter,
//! an animation engine, input state tracking, a widget trait, and the
//! concrete widgets that make up Moce.

pub mod animation;
pub mod context;
pub mod input;
pub mod painter;
pub mod theme;
pub mod widget;
pub mod widgets;

pub use animation::{Anim, AnimState, AnimationManager};
pub use context::UiContext;
pub use input::InputState;
pub use painter::{DrawCtx, Fonts};
pub use theme::Theme;
pub use widget::{EventResult, Widget};
