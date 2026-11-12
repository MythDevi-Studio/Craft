//! Input state tracking for the UI layer.
//!
//! The `InputState` accumulates events between frames so widgets can query
//! the current mouse position, button states, and keyboard modifiers without
//! each widget needing its own event handling.

use crate::math::Vec2;

/// Mouse button identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

impl MouseButton {
    /// Convert from a winit button index.
    pub fn from_winit(button: u16) -> Self {
        match button {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            n => MouseButton::Other(n),
        }
    }
}

/// Keyboard modifier keys state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl Modifiers {
    /// Whether any modifier is active.
    pub fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.super_key
    }
}

/// The complete input state for the current frame.
///
/// This is a snapshot taken at the start of each frame. Widgets read from
/// it; they should not mutate it.
pub struct InputState {
    /// Mouse position in window coordinates.
    pub mouse_pos: Vec2,
    /// Mouse position from the previous frame (for computing deltas).
    pub mouse_pos_prev: Vec2,
    /// Whether the mouse is inside the window.
    pub mouse_inside: bool,
    /// Set of currently held mouse buttons.
    pub mouse_buttons: MouseButtons,
    /// Mouse wheel scroll delta from this frame.
    pub scroll_delta: Vec2,
    /// Current keyboard modifier state.
    pub modifiers: Modifiers,
    /// Characters typed this frame (for text input).
    pub text_input: String,
    /// Whether text input was received this frame.
    pub has_text_input: bool,
    /// Window size in pixels.
    pub window_size: crate::math::Size,
}

impl InputState {
    /// Create a new empty input state.
    pub fn new(window_size: crate::math::Size) -> Self {
        Self {
            mouse_pos: Vec2::ZERO,
            mouse_pos_prev: Vec2::ZERO,
            mouse_inside: false,
            mouse_buttons: MouseButtons::default(),
            scroll_delta: Vec2::ZERO,
            modifiers: Modifiers::default(),
            text_input: String::new(),
            has_text_input: false,
            window_size,
        }
    }

    /// Whether a specific mouse button is currently held.
    #[inline]
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_buttons.left,
            MouseButton::Right => self.mouse_buttons.right,
            MouseButton::Middle => self.mouse_buttons.middle,
            MouseButton::Other(_) => false,
        }
    }

    /// Mouse movement delta since the last frame.
    #[inline]
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_pos - self.mouse_pos_prev
    }

    /// Begin a new frame — resets per-frame state.
    pub fn begin_frame(&mut self) {
        self.mouse_pos_prev = self.mouse_pos;
        self.scroll_delta = Vec2::ZERO;
        self.text_input.clear();
        self.has_text_input = false;
    }

    /// Set the window size (on resize).
    pub fn resize(&mut self, size: crate::math::Size) {
        self.window_size = size;
    }
}

/// Track which mouse buttons are held.
#[derive(Clone, Copy, Debug, Default)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}
