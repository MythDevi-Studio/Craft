//! Custom title bar widget for the frameless window.
//!
//! Renders the application name, the window control buttons (minimize,
//! maximize, close) and an optional breadcrumb path.
//!
//! # Drag-to-move
//! Because we use a frameless window, moving the window is handled via
//! winit's `window.drag_window()` API. The title bar emits a
//! `TitleBarAction::StartDrag` immediately on `MouseDown` on the draggable
//! area; the app calls `drag_window()` in response.

use crate::math::{Color, Lerp, Rect, Vec2};
use crate::ui::animation::Anim;
use crate::ui::widget::{EventResult, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

/// Which title-bar button is hovered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ControlBtn {
    Close,
    Minimize,
    Maximize,
}

/// Actions emitted by the title bar that the app must act on immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleBarEvent {
    Close,
    Minimize,
    Maximize,
    /// App must call `window.drag_window()` to begin an OS-native window drag.
    StartDrag,
}

pub struct TitleBar {
    pub title: String,
    /// Optional subtitle shown next to the title (e.g. current file path).
    pub subtitle: String,

    hovered_btn: Option<ControlBtn>,
    pressed_btn: Option<ControlBtn>,

    close_hover: Anim<f32>,
    max_hover: Anim<f32>,
    min_hover: Anim<f32>,

    pub events: Vec<TitleBarEvent>,
}

impl TitleBar {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: String::new(),
            hovered_btn: None,
            pressed_btn: None,
            close_hover: Anim::new(0.0),
            max_hover: Anim::new(0.0),
            min_hover: Anim::new(0.0),
            events: Vec::new(),
        }
    }

    pub fn take_events(&mut self) -> Vec<TitleBarEvent> {
        std::mem::take(&mut self.events)
    }

    /// Rects for the three macOS-style control dots.
    fn control_rects(rect: Rect) -> [(ControlBtn, Rect); 3] {
        // Larger hit area than visual size for easier clicking
        let visual  = 12.0_f32;
        let hit     = 20.0_f32;   // bigger hit box
        let gap     = 8.0_f32;
        let start_x = rect.x + 14.0;
        let cy      = rect.y + rect.h / 2.0;

        [
            (ControlBtn::Close,    Rect::new(start_x,                       cy - hit/2.0, hit, hit)),
            (ControlBtn::Minimize, Rect::new(start_x + visual + gap,        cy - hit/2.0, hit, hit)),
            (ControlBtn::Maximize, Rect::new(start_x + (visual + gap) * 2.0, cy - hit/2.0, hit, hit)),
        ]
    }

    /// Whether `pos` is in the draggable area (title bar minus buttons and
    /// the right 150 px reserved for future buttons).
    fn is_drag_area(pos: Vec2, rect: Rect) -> bool {
        if !rect.contains(pos) {
            return false;
        }
        // Exclude button strip on the left (~80 px)
        let btn_end_x = rect.x + 14.0 + 12.0 * 3.0 + 8.0 * 3.0 + 10.0;
        pos.x > btn_end_x
    }
}

impl Widget for TitleBar {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, fonts: &Fonts) {
        // ── Background ────────────────────────────────────────────────
        ctx.fill_rect(rect, theme.bg_title);
        ctx.draw_hline(rect.x, rect.bottom() - 1.0, rect.w, theme.border, 1.0);

        // ── Control dots ──────────────────────────────────────────────
        let controls = Self::control_rects(rect);

        let dot_colors = [
            Color::rgb(255, 95, 86),   // close  — red
            Color::rgb(255, 189, 46),  // min    — yellow
            Color::rgb(39, 201, 63),   // max    — green
        ];
        let hover_vals = [
            self.close_hover.value(),
            self.min_hover.value(),
            self.max_hover.value(),
        ];

        // Visual center of each dot (smaller than hit area)
        let visual = 12.0_f32;
        let gap    = 8.0_f32;
        let start_x = rect.x + 14.0;
        let cy = rect.y + rect.h / 2.0;
        let dot_centers = [
            Vec2::new(start_x + visual / 2.0,                  cy),
            Vec2::new(start_x + visual + gap + visual / 2.0,   cy),
            Vec2::new(start_x + (visual + gap) * 2.0 + visual / 2.0, cy),
        ];

        for i in 0..3 {
            let base    = dot_colors[i];
            let dimmed  = base.adjust_lightness(-0.15);
            let hovered = base;
            let color   = dimmed.lerp(&hovered, hover_vals[i]);
            ctx.fill_circle(dot_centers[i], visual / 2.0, color);

            // Show icon hint on hover
            if hover_vals[i] > 0.5 {
                let c = dot_centers[i];
                let r = 2.5_f32;
                let icon_col = Color::rgba(0, 0, 0, 120);
                match i {
                    0 => { // close ×
                        ctx.draw_line(Vec2::new(c.x-r, c.y-r), Vec2::new(c.x+r, c.y+r), icon_col, 1.5);
                        ctx.draw_line(Vec2::new(c.x+r, c.y-r), Vec2::new(c.x-r, c.y+r), icon_col, 1.5);
                    }
                    1 => { // minimize —
                        ctx.draw_hline(c.x - r, c.y, r * 2.0, icon_col, 1.5);
                    }
                    2 => { // maximize □
                        ctx.stroke_rect(Rect::new(c.x-r, c.y-r, r*2.0, r*2.0), icon_col, 1.0);
                    }
                    _ => {}
                }
            }
        }

        // ── Title text ────────────────────────────────────────────────
        let title_w = fonts.measure_ui(&self.title);
        let title_x = rect.x + (rect.w - title_w) / 2.0;
        let title_y = rect.y + (rect.h - fonts.ui.size()) / 2.0;
        ctx.draw_text(
            &self.title,
            Vec2::new(title_x, title_y),
            &fonts.ui_bold,
            theme.fg_secondary,
        );

        // ── Subtitle ──────────────────────────────────────────────────
        if !self.subtitle.is_empty() {
            let sub_x = title_x + title_w + 10.0;
            if sub_x + 60.0 < rect.right() - 100.0 {
                ctx.draw_text_clipped(
                    &self.subtitle,
                    Vec2::new(sub_x, title_y + 1.0),
                    &fonts.ui,
                    theme.fg_muted,
                    Rect::new(sub_x, rect.y, rect.right() - sub_x - 100.0, rect.h),
                );
            }
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _theme: &Theme) -> EventResult {
        let controls = Self::control_rects(rect);

        match event {
            // ── Hover ─────────────────────────────────────────────────
            UiEvent::MouseMove { pos } => {
                let mut new_hovered: Option<ControlBtn> = None;
                for (btn, hit) in &controls {
                    if hit.contains(*pos) {
                        new_hovered = Some(*btn);
                        break;
                    }
                }
                if new_hovered != self.hovered_btn {
                    self.hovered_btn = new_hovered;
                    let (c, mi, mx) = (
                        if new_hovered == Some(ControlBtn::Close)    { 1.0 } else { 0.0 },
                        if new_hovered == Some(ControlBtn::Minimize) { 1.0 } else { 0.0 },
                        if new_hovered == Some(ControlBtn::Maximize) { 1.0 } else { 0.0 },
                    );
                    self.close_hover.animate_to(c,  0.08);
                    self.min_hover  .animate_to(mi, 0.08);
                    self.max_hover  .animate_to(mx, 0.08);
                }
                EventResult::Ignored // don't consume — let other widgets also see move
            }

            // ── Press ─────────────────────────────────────────────────
            UiEvent::MouseDown { pos, button } if *button == MouseButton::Left => {
                // Check buttons first
                for (btn, hit) in &controls {
                    if hit.contains(*pos) {
                        self.pressed_btn = Some(*btn);
                        return EventResult::Consumed;
                    }
                }
                // Draggable area
                if Self::is_drag_area(*pos, rect) {
                    // Emit immediately — app must call drag_window() NOW
                    // (winit requires it to be called in the same event handler)
                    self.events.push(TitleBarEvent::StartDrag);
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            }

            // ── Release ───────────────────────────────────────────────
            UiEvent::MouseUp { pos, button } if *button == MouseButton::Left => {
                if let Some(pressed) = self.pressed_btn.take() {
                    // Fire only if still inside the same button's hit area
                    for (btn, hit) in &controls {
                        if *btn == pressed && hit.contains(*pos) {
                            let ev = match btn {
                                ControlBtn::Close    => TitleBarEvent::Close,
                                ControlBtn::Minimize => TitleBarEvent::Minimize,
                                ControlBtn::Maximize => TitleBarEvent::Maximize,
                            };
                            self.events.push(ev);
                            return EventResult::Consumed;
                        }
                    }
                }
                EventResult::Ignored
            }

            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        let a = self.close_hover.update(dt);
        let b = self.min_hover  .update(dt);
        let c = self.max_hover  .update(dt);
        a || b || c
    }
}
