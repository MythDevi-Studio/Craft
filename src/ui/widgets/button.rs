//! A clickable button widget with hover and press animations.

use crate::math::{Lerp, Rect, Vec2};
use crate::ui::animation::Anim;
use crate::ui::widget::{EventResult, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

/// A simple labeled button with animated hover/press feedback.
pub struct Button {
    pub label: String,
    /// Whether the button is enabled (can receive clicks).
    pub enabled: bool,
    /// Whether the button is currently hovered.
    hover: bool,
    /// Whether the button is currently pressed.
    pressed: bool,
    /// Animation progress for hover (0.0 = normal, 1.0 = fully hovered).
    hover_anim: Anim<f32>,
    /// Animation progress for press.
    press_anim: Anim<f32>,
    /// Callback called when the button is clicked.
    on_click: Option<Box<dyn FnMut()>>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            hover: false,
            pressed: false,
            hover_anim: Anim::new(0.0),
            press_anim: Anim::new(0.0),
            on_click: None,
        }
    }

    pub fn on_click(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }

    pub fn enabled(mut self, v: bool) -> Self {
        self.enabled = v;
        self
    }
}

impl Widget for Button {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, fonts: &Fonts) {
        let hover_t = self.hover_anim.value();
        let press_t = self.press_anim.value();

        // Background: blend between normal, hover, and press colors
        let bg = if !self.enabled {
            theme.bg_surface
        } else {
            let base = theme.bg_surface;
            let hovered = theme.bg_hover;
            let pressed = theme.bg_active;
            let after_hover = base.lerp(&hovered, hover_t);
            after_hover.lerp(&pressed, press_t)
        };

        ctx.fill_rrect(rect, theme.radius, bg);
        ctx.stroke_rrect(rect, theme.radius, theme.border, 1.0);

        // Label
        let text_color = if self.enabled {
            theme.fg_primary
        } else {
            theme.fg_muted
        };
        let label_w = fonts.measure_ui(&self.label);
        let label_x = rect.x + (rect.w - label_w) / 2.0;
        let label_y = rect.y + (rect.h - fonts.ui.size()) / 2.0;
        ctx.draw_text(&self.label, Vec2::new(label_x, label_y), &fonts.ui, text_color);
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _theme: &Theme) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }
        match event {
            UiEvent::MouseMove { pos } => {
                let now_inside = rect.contains(*pos);
                if now_inside != self.hover {
                    self.hover = now_inside;
                    if now_inside {
                        self.hover_anim.animate_to(1.0, 0.12);
                    } else {
                        self.hover_anim.animate_to(0.0, 0.18);
                        self.press_anim.animate_to(0.0, 0.1);
                    }
                }
                EventResult::Ignored
            }
            UiEvent::MouseDown { pos, button } if *button == MouseButton::Left => {
                if rect.contains(*pos) {
                    self.pressed = true;
                    self.press_anim.animate_to(1.0, 0.07);
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }
            UiEvent::MouseUp { pos, button } if *button == MouseButton::Left => {
                if self.pressed {
                    self.pressed = false;
                    self.press_anim.animate_to(0.0, 0.15);
                    if rect.contains(*pos) {
                        if let Some(cb) = &mut self.on_click {
                            cb();
                        }
                        return EventResult::Consumed;
                    }
                }
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        let a = self.hover_anim.update(dt);
        let b = self.press_anim.update(dt);
        a || b
    }
}

// We need a Lerp impl for Color to compile the lerp calls above.
// That's already in crate::math::color — just need a trait import here.

