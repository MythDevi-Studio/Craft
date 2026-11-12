//! Minimap widget — a scaled-down view of the entire document shown as a
//! scroll thumbnail on the right edge of the editor.
//!
//! Renders each line as a strip of colored pixels using the syntax token
//! colors. The viewport indicator (the visible region) is drawn as a
//! semi-transparent rectangle that the user can drag to scroll.

use crate::math::{Color, Rect, Vec2};
use crate::syntax::Highlighter;
use crate::ui::widget::{EventResult, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

pub struct Minimap {
    pub visible: bool,
    dragging: bool,
    drag_start_y: f32,
    drag_start_scroll: f32,
}

impl Minimap {
    pub fn new() -> Self {
        Self {
            visible: true,
            dragging: false,
            drag_start_y: 0.0,
            drag_start_scroll: 0.0,
        }
    }

    /// Compute the viewport indicator rectangle within the minimap.
    fn viewport_rect(
        &self,
        minimap_rect: Rect,
        total_lines: usize,
        scroll_y: f32,
        line_height: f32,
        viewport_h: f32,
    ) -> Rect {
        if total_lines == 0 { return Rect::ZERO; }
        let total_h = total_lines as f32 * line_height;
        let ratio = minimap_rect.h / total_h;
        let vp_h = (viewport_h * ratio).max(16.0).min(minimap_rect.h);
        let vp_y = minimap_rect.y + (scroll_y * ratio).min(minimap_rect.h - vp_h);
        Rect::new(minimap_rect.x, vp_y, minimap_rect.w, vp_h)
    }
}

impl Widget for Minimap {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, _fonts: &Fonts) {
        if !self.visible { return; }

        // Background
        ctx.fill_rect(rect, theme.bg_surface.adjust_lightness(-0.03));
        ctx.draw_vline(rect.x, rect.y, rect.h, theme.border_subtle, 1.0);

        // We don't have direct access to the buffer/highlighter here.
        // The EditorView will call paint_minimap_lines separately.
        // This paint() just draws the chrome (background, viewport indicator).
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _theme: &Theme) -> EventResult {
        if !self.visible { return EventResult::Ignored; }
        match event {
            UiEvent::MouseDown { pos, button } if *button == MouseButton::Left => {
                if rect.contains(*pos) {
                    self.dragging = true;
                    self.drag_start_y = pos.y;
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            }
            UiEvent::MouseUp { button, .. } if *button == MouseButton::Left => {
                self.dragging = false;
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }
}

/// Draw the minimap content lines into a rect.
/// This is a free function so the editor view can call it with buffer access.
pub fn paint_minimap_content(
    ctx: &DrawCtx<'_>,
    rect: Rect,
    highlighter: &mut Highlighter,
    buffer: &crate::editor::Buffer,
    scroll_y: f32,
    viewport_h: f32,
    theme: &Theme,
) {
    let px_per_line = 2.0_f32; // each line = 2px tall in the minimap
    let char_w = 1.5_f32;
    let total_lines = buffer.len_lines();
    let max_lines = (rect.h / px_per_line) as usize;

    // Determine which lines to show (the range that fits in rect.h)
    let total_doc_h = total_lines as f32 * theme.line_height_px();
    let first_line_f = if total_doc_h > 0.0 {
        (scroll_y / total_doc_h * total_lines as f32).max(0.0)
    } else {
        0.0
    };
    let first_line = first_line_f as usize;
    let last_line = (first_line + max_lines).min(total_lines);

    for (i, line_idx) in (first_line..last_line).enumerate() {
        let ly = rect.y + i as f32 * px_per_line;
        let line_text = buffer.line(line_idx);
        if line_text.is_empty() { continue; }

        let tokens = highlighter.tokenize_line(&line_text);
        for token in &tokens {
            let start = token.start.min(line_text.len());
            let end = token.end.min(line_text.len());
            if start >= end { continue; }
            let char_col = line_text[..start].chars().count();
            let char_len = line_text[start..end].chars().count();
            let tx = rect.x + char_col as f32 * char_w;
            if tx > rect.right() { break; }
            let tw = (char_len as f32 * char_w).min(rect.right() - tx);

            let base_color = theme.syntax_color(token.kind.theme_key());
            let mini_color = Color::rgba(base_color.r, base_color.g, base_color.b, 100);
            ctx.fill_rect(Rect::new(tx, ly, tw, px_per_line.max(1.0)), mini_color);
        }
    }

    // Draw viewport indicator
    let total_doc_px = total_lines as f32 * px_per_line;
    let vp_ratio = viewport_h / (total_lines as f32 * theme.line_height_px()).max(1.0);
    let vp_h = (vp_ratio * total_doc_px).max(8.0).min(rect.h);
    let vp_offset = (scroll_y / total_doc_h.max(1.0)) * total_doc_px;
    let vp_y = rect.y + vp_offset.min(rect.h - vp_h);
    ctx.fill_rect(
        Rect::new(rect.x, vp_y, rect.w, vp_h),
        Color::rgba(255, 255, 255, 18),
    );
    ctx.stroke_rect(
        Rect::new(rect.x, vp_y, rect.w, vp_h),
        Color::rgba(255, 255, 255, 40),
        1.0,
    );
}
