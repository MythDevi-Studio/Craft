//! UiContext — the shared state passed through the entire render/event tree.
//!
//! Rather than threading `theme`, `fonts`, and `input` through every
//! function signature, they live in one `UiContext` struct. Widgets receive
//! a reference to it and can read what they need.

use crate::ui::{Fonts, InputState, Theme};

/// Shared UI context passed to every widget during paint and event handling.
pub struct UiContext {
    pub theme: Theme,
    pub fonts: Fonts,
    pub input: InputState,
    pub scale_factor: f32,
}

impl UiContext {
    pub fn new(theme: Theme, fonts: Fonts, window_size: crate::math::Size) -> Self {
        Self {
            theme,
            fonts,
            input: InputState::new(window_size),
            scale_factor: 1.0,
        }
    }

    /// Begin a new frame — resets per-frame input state.
    pub fn begin_frame(&mut self) {
        self.input.begin_frame();
    }

    /// Convert a logical pixel size to physical pixels using the scale factor.
    #[inline]
    pub fn to_physical(&self, logical: f32) -> f32 {
        logical * self.scale_factor
    }

    /// Convert physical pixels back to logical pixels.
    #[inline]
    pub fn to_logical(&self, physical: f32) -> f32 {
        physical / self.scale_factor
    }
}
