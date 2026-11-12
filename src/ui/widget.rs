//! Widget trait and event result type.
//!
//! Every UI element in Moce is a `Widget`. Widgets receive input events,
//! report whether they consumed them, and paint themselves into the
//! `DrawCtx`. The layout is explicit: each widget is given a `Rect` that
//! tells it exactly where to draw. There's no automatic layout engine —
//! layout is computed by parent widgets or the main app loop, which keeps
//! things fast and predictable.

use crate::math::{Rect, Size, Vec2};
use crate::ui::{DrawCtx, Fonts, InputState, Theme};

/// The result returned by a widget after handling an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventResult {
    /// The event was consumed — stop propagation.
    Consumed,
    /// The event was not handled — continue propagation.
    Ignored,
}

impl EventResult {
    pub fn is_consumed(&self) -> bool {
        *self == EventResult::Consumed
    }
}

/// An input event delivered to widgets.
#[derive(Debug, Clone)]
pub enum UiEvent {
    MouseMove { pos: Vec2 },
    MouseDown { pos: Vec2, button: MouseButton },
    MouseUp { pos: Vec2, button: MouseButton },
    Scroll { delta: Vec2, pos: Vec2 },
    KeyDown { key: Key, modifiers: Modifiers },
    KeyUp { key: Key, modifiers: Modifiers },
    TextInput(String),
    Resize(Size),
    FocusGained,
    FocusLost,
}

pub use crate::ui::input::{Modifiers, MouseButton};

/// Keyboard key identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Enter,
    Escape,
    Backspace,
    Delete,
    Tab,
    F(u32),
    Ctrl,
    Alt,
    Shift,
    Super,
    Unknown,
}

/// The common widget interface.
///
/// Widgets are painted and updated via explicit calls from their parent.
/// There is no retained widget graph — parents own children directly
/// (usually as struct fields or `Vec<Box<dyn Widget>>`).
pub trait Widget {
    /// Paint this widget into the given draw context.
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, fonts: &Fonts);

    /// Handle an input event. Return `Consumed` if the widget used it.
    fn on_event(
        &mut self,
        event: &UiEvent,
        rect: Rect,
        theme: &Theme,
    ) -> EventResult {
        let _ = (event, rect, theme);
        EventResult::Ignored
    }

    /// Called every frame. Return true if the widget needs a repaint
    /// (e.g., because an animation is running).
    fn update(&mut self, _dt: f32) -> bool {
        false
    }
}

/// Helper: check if a mouse event is inside a given rect.
pub fn hit_test(pos: Vec2, rect: Rect) -> bool {
    rect.contains(pos)
}
