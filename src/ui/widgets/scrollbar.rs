//! Scrollbar widget — a minimal, animated scroll thumb.

use crate::math::{Color, Lerp, Rect, Vec2};
use crate::ui::animation::Anim;
use crate::ui::widget::{EventResult, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

/// Orientation of the scrollbar.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarOrientation {
    Vertical,
    Horizontal,
}

/// A slim, auto-hiding scrollbar widget.
pub struct Scrollbar {
    pub orientation: ScrollbarOrientation,
    /// Total content size in the scroll direction.
    pub content_size: f32,
    /// Visible viewport size in the scroll direction.
    pub viewport_size: f32,
    /// Current scroll position (content offset).
    pub scroll: f32,
    hover: bool,
    dragging: bool,
    drag_start_pos: f32,
    drag_start_scroll: f32,
    /// Fade-in/out animation for auto-hide.
    opacity: Anim<f32>,
}

impl Scrollbar {
    pub fn new(orientation: ScrollbarOrientation) -> Self {
        Self {
            orientation,
            content_size: 1.0,
            viewport_size: 1.0,
            scroll: 0.0,
            hover: false,
            dragging: false,
            drag_start_pos: 0.0,
            drag_start_scroll: 0.0,
            opacity: Anim::new(0.0),
        }
    }

    /// Whether the scrollbar is needed (content exceeds viewport).
    pub fn needed(&self) -> bool {
        self.content_size > self.viewport_size
    }

    /// Clamp scroll to valid range.
    pub fn clamp_scroll(&mut self) {
        let max = (self.content_size - self.viewport_size).max(0.0);
        self.scroll = self.scroll.clamp(0.0, max);
    }

    /// Show the scrollbar (fade in).
    pub fn show(&mut self) {
        self.opacity.animate_to(1.0, 0.1);
    }

    /// Hide the scrollbar (fade out).
    pub fn hide(&mut self) {
        if !self.dragging && !self.hover {
            self.opacity.animate_to(0.0, 0.5);
        }
    }

    fn thumb_rect(&self, track: Rect) -> Rect {
        if self.content_size <= 0.0 {
            return Rect::ZERO;
        }
        let ratio = self.viewport_size / self.content_size;
        let scroll_ratio = self.scroll / self.content_size;

        match self.orientation {
            ScrollbarOrientation::Vertical => {
                let thumb_h = (track.h * ratio).max(32.0).min(track.h);
                let thumb_y = track.y + (track.h - thumb_h) * scroll_ratio;
                Rect::new(track.x + 2.0, thumb_y, track.w - 4.0, thumb_h)
            }
            ScrollbarOrientation::Horizontal => {
                let thumb_w = (track.w * ratio).max(32.0).min(track.w);
                let thumb_x = track.x + (track.w - thumb_w) * scroll_ratio;
                Rect::new(thumb_x, track.y + 2.0, thumb_w, track.h - 4.0)
            }
        }
    }

    fn pos_component(&self, pos: Vec2) -> f32 {
        match self.orientation {
            ScrollbarOrientation::Vertical => pos.y,
            ScrollbarOrientation::Horizontal => pos.x,
        }
    }
}

impl Widget for Scrollbar {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, _fonts: &Fonts) {
        if !self.needed() {
            return;
        }
        let alpha = self.opacity.value();
        if alpha < 0.01 {
            return;
        }

        let thumb = self.thumb_rect(rect);
        let base_color = if self.hover || self.dragging {
            theme.fg_secondary
        } else {
            theme.fg_muted
        };
        // Apply opacity to alpha channel
        let color = Color::rgba(
            base_color.r,
            base_color.g,
            base_color.b,
            (base_color.a as f32 * alpha) as u8,
        );
        ctx.fill_rrect(thumb, (thumb.w / 2.0).min(thumb.h / 2.0), color);
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _theme: &Theme) -> EventResult {
        if !self.needed() {
            return EventResult::Ignored;
        }
        match event {
            UiEvent::MouseMove { pos } => {
                let now_hover = rect.contains(*pos);
                if now_hover != self.hover {
                    self.hover = now_hover;
                    if now_hover {
                        self.show();
                    }
                }
                if self.dragging {
                    let pos_component = self.pos_component(*pos);
                    let delta = pos_component - self.drag_start_pos;
                    let track_len = match self.orientation {
                        ScrollbarOrientation::Vertical => rect.h,
                        ScrollbarOrientation::Horizontal => rect.w,
                    };
                    let scroll_range = (self.content_size - self.viewport_size).max(0.0);
                    let scroll_delta = delta * scroll_range / (track_len - self.thumb_rect(rect).h.max(self.thumb_rect(rect).w));
                    self.scroll = (self.drag_start_scroll + scroll_delta).clamp(0.0, scroll_range);
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            }
            UiEvent::MouseDown { pos, button } if *button == MouseButton::Left => {
                if rect.contains(*pos) {
                    let thumb = self.thumb_rect(rect);
                    if thumb.contains(*pos) {
                        self.dragging = true;
                        self.drag_start_pos = self.pos_component(*pos);
                        self.drag_start_scroll = self.scroll;
                    } else {
                        // Click on track — jump to position
                        let pos_c = self.pos_component(*pos);
                        let track_start = match self.orientation {
                            ScrollbarOrientation::Vertical => rect.y,
                            ScrollbarOrientation::Horizontal => rect.x,
                        };
                        let track_len = match self.orientation {
                            ScrollbarOrientation::Vertical => rect.h,
                            ScrollbarOrientation::Horizontal => rect.w,
                        };
                        let t = ((pos_c - track_start) / track_len).clamp(0.0, 1.0);
                        self.scroll = t * (self.content_size - self.viewport_size).max(0.0);
                    }
                    self.show();
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            }
            UiEvent::MouseUp { button, .. } if *button == MouseButton::Left => {
                self.dragging = false;
                self.hide();
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        self.opacity.update(dt)
    }
}
