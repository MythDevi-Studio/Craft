//! Toast notifications — transient messages that appear in the bottom-right
//! corner and fade out automatically.

use crate::math::{Color, Rect, Vec2};
use crate::ui::animation::Anim;
use crate::ui::{DrawCtx, Fonts, Theme};

/// The severity level of a notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationKind {
    fn accent(self, theme: &Theme) -> Color {
        match self {
            NotificationKind::Info => theme.accent,
            NotificationKind::Success => Color::rgb(39, 201, 63),
            NotificationKind::Warning => Color::rgb(255, 189, 46),
            NotificationKind::Error => Color::rgb(255, 95, 86),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            NotificationKind::Info => "ℹ",
            NotificationKind::Success => "✓",
            NotificationKind::Warning => "⚠",
            NotificationKind::Error => "✕",
        }
    }
}

struct Toast {
    message: String,
    kind: NotificationKind,
    opacity: Anim<f32>,
    slide: Anim<f32>,
    lifetime: f32, // seconds remaining
}

impl Toast {
    fn new(message: String, kind: NotificationKind, duration: f32) -> Self {
        let mut opacity = Anim::new(0.0);
        let mut slide = Anim::new(60.0);
        opacity.animate_to(1.0, 0.15);
        slide.animate_to(0.0, 0.18);
        Self { message, kind, opacity, slide, lifetime: duration }
    }

    fn start_dismiss(&mut self) {
        self.opacity.animate_to(0.0, 0.25);
        self.slide.animate_to(40.0, 0.2);
    }

    fn is_dead(&self) -> bool {
        self.opacity.value() < 0.01 && self.lifetime <= 0.0
    }

    fn update(&mut self, dt: f32) -> bool {
        self.lifetime -= dt;
        if self.lifetime <= 0.0 && self.opacity.value() > 0.5 {
            self.start_dismiss();
        }
        let a = self.opacity.update(dt);
        let b = self.slide.update(dt);
        a || b
    }
}

/// Manages all active toast notifications.
pub struct NotificationManager {
    toasts: Vec<Toast>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self { toasts: Vec::new() }
    }

    /// Show a toast. `duration` is seconds before it fades out.
    pub fn push(&mut self, message: impl Into<String>, kind: NotificationKind, duration: f32) {
        // Limit to 4 visible at once
        if self.toasts.len() >= 4 {
            if let Some(oldest) = self.toasts.first_mut() {
                oldest.start_dismiss();
            }
        }
        self.toasts.push(Toast::new(message.into(), kind, duration));
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.push(msg, NotificationKind::Info, 3.5);
    }

    pub fn success(&mut self, msg: impl Into<String>) {
        self.push(msg, NotificationKind::Success, 2.5);
    }

    pub fn warning(&mut self, msg: impl Into<String>) {
        self.push(msg, NotificationKind::Warning, 4.0);
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.push(msg, NotificationKind::Error, 6.0);
    }

    /// Update all toasts and remove dead ones. Returns true if any are animating.
    pub fn update(&mut self, dt: f32) -> bool {
        let mut any = false;
        for t in &mut self.toasts {
            if t.update(dt) { any = true; }
        }
        self.toasts.retain(|t| !t.is_dead());
        any
    }

    /// Paint all active toasts into the window rect.
    pub fn paint(&self, ctx: &DrawCtx<'_>, window_rect: Rect, theme: &Theme, fonts: &Fonts) {
        const TOAST_W: f32 = 320.0;
        const TOAST_H: f32 = 52.0;
        const GAP: f32 = 8.0;
        const MARGIN_RIGHT: f32 = 16.0;
        const MARGIN_BOTTOM: f32 = 32.0;

        let base_x = window_rect.right() - TOAST_W - MARGIN_RIGHT;
        let base_y = window_rect.bottom() - MARGIN_BOTTOM;

        for (i, toast) in self.toasts.iter().enumerate().rev() {
            let alpha = toast.opacity.value();
            if alpha < 0.01 { continue; }
            let slide = toast.slide.value();

            let y = base_y - (i as f32 + 1.0) * (TOAST_H + GAP) - slide;
            let x = base_x;
            let rect = Rect::new(x, y, TOAST_W, TOAST_H);

            let opacity = |c: Color| Color::rgba(c.r, c.g, c.b, (c.a as f32 * alpha) as u8);

            // Shadow
            ctx.draw_shadow(rect, 8.0, 12.0, Color::rgba(0, 0, 0, (60.0 * alpha) as u8));

            // Background
            ctx.fill_rrect(rect, 8.0, opacity(theme.bg_elevated));
            ctx.stroke_rrect(rect, 8.0, opacity(theme.border), 1.0);

            // Left accent bar
            let accent = toast.kind.accent(theme);
            ctx.fill_rrect(Rect::new(x, y, 4.0, TOAST_H), 2.0, opacity(accent));

            // Icon
            let icon = toast.kind.icon();
            let icon_y = y + (TOAST_H - fonts.ui.size()) / 2.0;
            ctx.draw_text(icon, Vec2::new(x + 14.0, icon_y), &fonts.ui, opacity(accent));

            // Message (wrap at TOAST_W - 50)
            let msg_x = x + 36.0;
            let max_text_w = TOAST_W - 50.0;
            // Simple single-line clipping for now
            ctx.draw_text_clipped(
                &toast.message,
                Vec2::new(msg_x, icon_y),
                &fonts.ui,
                opacity(theme.fg_primary),
                Rect::new(msg_x, y, max_text_w, TOAST_H),
            );
        }
    }

    pub fn has_active(&self) -> bool {
        !self.toasts.is_empty()
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}
