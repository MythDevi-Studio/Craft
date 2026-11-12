//! LSP popup widgets: hover tooltip and completion list.

use crate::lsp::types::{CompletionItem, Hover, HoverContents};
use crate::math::{Color, Rect, Vec2};
use crate::ui::animation::Anim;
use crate::ui::widget::{EventResult, Key, Modifiers, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

// ══════════════════════════════════════════════════════════════════════════════
// Hover popup
// ══════════════════════════════════════════════════════════════════════════════

pub struct HoverPopup {
    pub content: Option<String>,
    /// Position in screen coordinates.
    pub anchor: Vec2,
    opacity: Anim<f32>,
}

impl HoverPopup {
    pub fn new() -> Self {
        Self {
            content: None,
            anchor: Vec2::ZERO,
            opacity: Anim::new(0.0),
        }
    }

    pub fn show(&mut self, text: String, anchor: Vec2) {
        self.content = Some(text);
        self.anchor = anchor;
        self.opacity.animate_to(1.0, 0.1);
    }

    pub fn hide(&mut self) {
        self.opacity.animate_to(0.0, 0.08);
    }

    pub fn is_visible(&self) -> bool {
        self.opacity.value() > 0.01
    }
}

impl Widget for HoverPopup {
    fn paint(&mut self, ctx: &DrawCtx<'_>, _rect: Rect, theme: &Theme, fonts: &Fonts) {
        let alpha = self.opacity.value();
        if alpha < 0.01 { return; }

        let text = match &self.content {
            Some(t) => t.clone(),
            None => return,
        };

        let pad = 10.0;
        let max_w = 420.0;
        let lines: Vec<&str> = text.lines().take(12).collect();
        let lh = fonts.mono.size() * 1.4;
        let popup_w = lines.iter()
            .map(|l| fonts.measure_mono(l) + pad * 2.0)
            .fold(0.0_f32, f32::max)
            .min(max_w);
        let popup_h = lines.len() as f32 * lh + pad * 2.0;

        let x = self.anchor.x;
        let y = self.anchor.y - popup_h - 6.0;

        let popup_rect = Rect::new(x, y, popup_w, popup_h);

        let opacity = |c: Color| Color::rgba(c.r, c.g, c.b, (c.a as f32 * alpha) as u8);

        ctx.draw_shadow(popup_rect, 6.0, 12.0, Color::rgba(0, 0, 0, (60.0 * alpha) as u8));
        ctx.fill_rrect(popup_rect, 6.0, opacity(theme.bg_elevated));
        ctx.stroke_rrect(popup_rect, 6.0, opacity(theme.border), 1.0);

        // Top accent bar
        ctx.fill_rrect(Rect::new(x, y, popup_w, 3.0), 2.0, opacity(theme.accent));

        for (i, line) in lines.iter().enumerate() {
            let ly = y + pad + i as f32 * lh;
            ctx.draw_text_clipped(
                line,
                Vec2::new(x + pad, ly),
                &fonts.mono,
                opacity(theme.fg_primary),
                popup_rect,
            );
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        self.opacity.update(dt)
    }

    fn on_event(&mut self, event: &UiEvent, _rect: Rect, _theme: &Theme) -> EventResult {
        if let UiEvent::MouseMove { .. } = event {
            // Could implement auto-hide on mouse move away from anchor
        }
        EventResult::Ignored
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Completion popup
// ══════════════════════════════════════════════════════════════════════════════

pub struct CompletionPopup {
    pub items: Vec<CompletionItem>,
    pub anchor: Vec2,
    pub selected: usize,
    opacity: Anim<f32>,
    scroll_offset: usize,
    pub events: Vec<CompletionEvent>,
}

#[derive(Debug, Clone)]
pub enum CompletionEvent {
    Accepted(CompletionItem),
    Dismissed,
}

impl CompletionPopup {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            anchor: Vec2::ZERO,
            selected: 0,
            opacity: Anim::new(0.0),
            scroll_offset: 0,
            events: Vec::new(),
        }
    }

    pub fn show(&mut self, items: Vec<CompletionItem>, anchor: Vec2) {
        self.items = items;
        self.anchor = anchor;
        self.selected = 0;
        self.scroll_offset = 0;
        self.opacity.animate_to(1.0, 0.1);
    }

    pub fn hide(&mut self) {
        self.opacity.animate_to(0.0, 0.08);
        self.items.clear();
    }

    pub fn is_visible(&self) -> bool {
        self.opacity.value() > 0.01 && !self.items.is_empty()
    }

    pub fn take_events(&mut self) -> Vec<CompletionEvent> {
        std::mem::take(&mut self.events)
    }

    fn kind_icon(kind: Option<u8>) -> &'static str {
        match kind {
            Some(1) => "󰅩",  // Text
            Some(2) => "ƒ",  // Method
            Some(3) => "ƒ",  // Function
            Some(4) => "⊘",  // Constructor
            Some(5) => "⊡",  // Field
            Some(6) => "v",  // Variable
            Some(7) => "⊕",  // Class
            Some(8) => "I",  // Interface
            Some(9) => "M",  // Module
            Some(10) => "⊟", // Property
            Some(13) => "K", // Keyword
            _ => "·",
        }
    }

    fn kind_color(kind: Option<u8>, theme: &Theme) -> Color {
        match kind {
            Some(2) | Some(3) => theme.syntax_color("function"),
            Some(7) | Some(8) => theme.syntax_color("type"),
            Some(13) => theme.syntax_color("keyword"),
            Some(5) | Some(10) => theme.syntax_color("identifier"),
            _ => theme.fg_muted,
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let n = self.items.len();
        if n == 0 { return; }
        self.selected = ((self.selected as i32 + delta).rem_euclid(n as i32)) as usize;
        const VISIBLE: usize = 8;
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + VISIBLE {
            self.scroll_offset = self.selected + 1 - VISIBLE;
        }
    }

    fn accept_selected(&mut self) {
        if let Some(item) = self.items.get(self.selected) {
            self.events.push(CompletionEvent::Accepted(item.clone()));
            self.hide();
        }
    }
}

impl Widget for CompletionPopup {
    fn paint(&mut self, ctx: &DrawCtx<'_>, _rect: Rect, theme: &Theme, fonts: &Fonts) {
        let alpha = self.opacity.value();
        if alpha < 0.01 || self.items.is_empty() { return; }

        let opacity = |c: Color| Color::rgba(c.r, c.g, c.b, (c.a as f32 * alpha) as u8);

        const MAX_VISIBLE: usize = 8;
        let item_h = 24.0;
        let pad_x = 10.0;
        let popup_w = 320.0;
        let visible_count = self.items.len().min(MAX_VISIBLE);
        let popup_h = visible_count as f32 * item_h;

        let popup_rect = Rect::new(self.anchor.x, self.anchor.y, popup_w, popup_h);

        ctx.draw_shadow(popup_rect, 6.0, 10.0, Color::rgba(0, 0, 0, (50.0 * alpha) as u8));
        ctx.fill_rrect(popup_rect, 6.0, opacity(theme.bg_elevated));
        ctx.stroke_rrect(popup_rect, 6.0, opacity(theme.border), 1.0);

        for (vis_idx, item) in self.items[self.scroll_offset..].iter().take(MAX_VISIBLE).enumerate() {
            let abs_idx = self.scroll_offset + vis_idx;
            let iy = self.anchor.y + vis_idx as f32 * item_h;
            let item_rect = Rect::new(self.anchor.x, iy, popup_w, item_h);

            if abs_idx == self.selected {
                ctx.fill_rrect(
                    Rect::new(self.anchor.x + 2.0, iy + 1.0, popup_w - 4.0, item_h - 2.0),
                    4.0,
                    opacity(theme.bg_hover),
                );
            }

            let ty = iy + (item_h - fonts.mono.size()) / 2.0;

            // Kind icon
            let icon = Self::kind_icon(item.kind);
            let icon_color = Self::kind_color(item.kind, theme);
            ctx.draw_text(icon, Vec2::new(self.anchor.x + pad_x, ty), &fonts.ui, opacity(icon_color));

            // Label
            let label_color = if abs_idx == self.selected {
                opacity(theme.fg_primary)
            } else {
                opacity(theme.fg_secondary)
            };
            ctx.draw_text_clipped(
                &item.label,
                Vec2::new(self.anchor.x + pad_x + 20.0, ty),
                &fonts.mono,
                label_color,
                item_rect,
            );

            // Detail
            if let Some(detail) = &item.detail {
                let dw = fonts.measure_ui(detail);
                ctx.draw_text_clipped(
                    detail,
                    Vec2::new(self.anchor.x + popup_w - dw - pad_x, ty),
                    &fonts.ui,
                    opacity(theme.fg_muted),
                    item_rect,
                );
            }
        }
    }

    fn on_event(&mut self, event: &UiEvent, _rect: Rect, _theme: &Theme) -> EventResult {
        if !self.is_visible() { return EventResult::Ignored; }
        match event {
            UiEvent::KeyDown { key, .. } => match key {
                Key::Up => { self.move_selection(-1); EventResult::Consumed }
                Key::Down => { self.move_selection(1); EventResult::Consumed }
                Key::Enter | Key::Tab => { self.accept_selected(); EventResult::Consumed }
                Key::Escape => {
                    self.events.push(CompletionEvent::Dismissed);
                    self.hide();
                    EventResult::Consumed
                }
                _ => EventResult::Ignored,
            }
            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        self.opacity.update(dt)
    }
}
