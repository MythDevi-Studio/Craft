//! Command palette — fuzzy-search over commands, files, and symbols.
//!
//! Triggered by Ctrl+Shift+P (commands) or Ctrl+P (files). Shows a
//! floating, centered overlay with a text field and a scrollable list of
//! matches, highlighted by the fuzzy match score.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::math::{Color, Rect, Size, Vec2};
use crate::ui::animation::Anim;
use crate::ui::widget::{EventResult, Key, Modifiers, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

/// A single item shown in the command palette list.
#[derive(Clone, Debug)]
pub struct PaletteItem {
    pub id: String,
    pub label: String,
    pub description: String,
    pub kind: PaletteKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteKind {
    Command,
    File,
    Symbol,
    RecentFile,
}

impl PaletteKind {
    fn icon(self) -> &'static str {
        match self {
            PaletteKind::Command => "⌘",
            PaletteKind::File => "󰈙",
            PaletteKind::Symbol => "⊕",
            PaletteKind::RecentFile => "󰋚",
        }
    }
}

/// An event emitted when the user picks an item.
#[derive(Debug, Clone)]
pub struct PaletteEvent {
    pub item_id: String,
    pub kind: PaletteKind,
}

/// The command palette overlay widget.
pub struct CommandPalette {
    pub visible: bool,
    query: String,
    all_items: Vec<PaletteItem>,
    filtered: Vec<(i64, usize)>, // (score, index into all_items)
    selected: usize,
    scroll_offset: usize,
    matcher: SkimMatcherV2,
    /// Fade-in/out animation.
    opacity: Anim<f32>,
    /// Scale animation (pop-in effect).
    scale: Anim<f32>,
    pub events: Vec<PaletteEvent>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            all_items: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            matcher: SkimMatcherV2::default(),
            opacity: Anim::new(0.0),
            scale: Anim::new(0.95),
            events: Vec::new(),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
        self.scroll_offset = 0;
        self.opacity.animate_to(1.0, 0.12);
        self.scale.animate_to(1.0, 0.14);
        self.update_filtered();
    }

    pub fn hide(&mut self) {
        self.opacity.animate_to(0.0, 0.1);
        self.scale.animate_to(0.96, 0.1);
        // visible = false is set after the animation completes
    }

    pub fn set_items(&mut self, items: Vec<PaletteItem>) {
        self.all_items = items;
        self.update_filtered();
    }

    pub fn take_events(&mut self) -> Vec<PaletteEvent> {
        std::mem::take(&mut self.events)
    }

    fn update_filtered(&mut self) {
        self.filtered.clear();
        if self.query.is_empty() {
            // Show all items sorted by kind
            for (i, item) in self.all_items.iter().enumerate() {
                self.filtered.push((0, i));
            }
        } else {
            for (i, item) in self.all_items.iter().enumerate() {
                let haystack = format!("{} {}", item.label, item.description);
                if let Some(score) = self.matcher.fuzzy_match(&haystack, &self.query) {
                    self.filtered.push((score, i));
                }
            }
            self.filtered.sort_by(|a, b| b.0.cmp(&a.0));
        }
        self.selected = 0;
        self.scroll_offset = 0;
    }

    fn confirm_selection(&mut self) {
        if let Some(&(_, idx)) = self.filtered.get(self.selected) {
            if let Some(item) = self.all_items.get(idx) {
                self.events.push(PaletteEvent {
                    item_id: item.id.clone(),
                    kind: item.kind,
                });
                self.hide();
            }
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let n = self.filtered.len();
        if n == 0 { return; }
        self.selected = ((self.selected as i32 + delta).rem_euclid(n as i32)) as usize;
        // Scroll into view
        const VISIBLE: usize = 8;
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + VISIBLE {
            self.scroll_offset = self.selected + 1 - VISIBLE;
        }
    }
}

impl Widget for CommandPalette {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, fonts: &Fonts) {
        let alpha = self.opacity.value();
        if alpha < 0.01 { return; }
        let scale = self.scale.value();

        // ── Backdrop ─────────────────────────────────────────────────
        let backdrop = Color::rgba(0, 0, 0, (180.0 * alpha) as u8);
        ctx.fill_rect(rect, backdrop);

        // ── Panel ────────────────────────────────────────────────────
        let panel_w = (rect.w * 0.55).max(500.0).min(700.0);
        let panel_x = rect.x + (rect.w - panel_w) / 2.0;
        let item_h = 36.0;
        let input_h = 48.0;
        let max_items: usize = 8;
        let visible_count = self.filtered.len().min(max_items);
        let panel_h = input_h + visible_count as f32 * item_h + 8.0;
        let panel_y = rect.y + rect.h * 0.18;

        // Scale from center
        let cx = panel_x + panel_w / 2.0;
        let cy = panel_y + panel_h / 2.0;
        ctx.canvas.save();
        ctx.canvas.translate((cx, cy));
        ctx.canvas.scale((scale, scale));
        ctx.canvas.translate((-cx, -cy));

        let opacity_color = |c: Color| -> Color {
            Color::rgba(c.r, c.g, c.b, (c.a as f32 * alpha) as u8)
        };

        let panel_rect = Rect::new(panel_x, panel_y, panel_w, panel_h);

        // Shadow
        ctx.draw_shadow(panel_rect, 10.0, 24.0, Color::rgba(0, 0, 0, (100.0 * alpha) as u8));

        // Background
        ctx.fill_rrect(panel_rect, 10.0, opacity_color(theme.bg_elevated));
        ctx.stroke_rrect(panel_rect, 10.0, opacity_color(theme.border), 1.0);

        // ── Input field ───────────────────────────────────────────────
        let input_rect = Rect::new(panel_x, panel_y, panel_w, input_h);
        let border_b = Color::rgba(theme.border.r, theme.border.g, theme.border.b, (theme.border.a as f32 * alpha) as u8);
        ctx.draw_hline(input_rect.x + 16.0, input_rect.bottom() - 1.0, panel_w - 32.0, border_b, 1.0);

        // Search icon
        let icon_x = panel_x + 16.0;
        let icon_y = panel_y + (input_h - fonts.ui.size()) / 2.0;
        ctx.draw_text("⌕", Vec2::new(icon_x, icon_y), &fonts.ui, opacity_color(theme.fg_muted));

        // Query text or placeholder
        let text_x = icon_x + 24.0;
        if self.query.is_empty() {
            ctx.draw_text("Type to search...", Vec2::new(text_x, icon_y), &fonts.ui, opacity_color(theme.fg_muted));
        } else {
            ctx.draw_text(&self.query, Vec2::new(text_x, icon_y), &fonts.mono, opacity_color(theme.fg_primary));
            // Cursor
            let cursor_x = text_x + fonts.measure_ui(&self.query);
            let cursor_y = panel_y + 10.0;
            ctx.fill_rect(Rect::new(cursor_x, cursor_y, 2.0, input_h - 20.0), opacity_color(theme.cursor));
        }

        // ── Items list ────────────────────────────────────────────────
        ctx.clip(Rect::new(panel_x, panel_y + input_h, panel_w, panel_h - input_h), |ctx| {
            let items_to_show = self.filtered[self.scroll_offset..]
                .iter()
                .take(max_items)
                .enumerate();

            for (vis_idx, &(score, item_idx)) in items_to_show {
                let item = &self.all_items[item_idx];
                let abs_idx = self.scroll_offset + vis_idx;
                let iy = panel_y + input_h + vis_idx as f32 * item_h;
                let item_rect = Rect::new(panel_x, iy, panel_w, item_h);

                // Selection highlight
                if abs_idx == self.selected {
                    ctx.fill_rrect(
                        Rect::new(panel_x + 4.0, iy + 2.0, panel_w - 8.0, item_h - 4.0),
                        6.0,
                        opacity_color(theme.bg_hover),
                    );
                    // Left indicator bar
                    ctx.fill_rrect(
                        Rect::new(panel_x + 4.0, iy + 8.0, 3.0, item_h - 16.0),
                        2.0,
                        opacity_color(theme.accent),
                    );
                }

                let text_y = iy + (item_h - fonts.ui.size()) / 2.0;

                // Icon
                ctx.draw_text(
                    item.kind.icon(),
                    Vec2::new(panel_x + 16.0, text_y),
                    &fonts.ui,
                    opacity_color(theme.fg_muted),
                );

                // Label
                let label_color = if abs_idx == self.selected {
                    opacity_color(theme.fg_primary)
                } else {
                    opacity_color(theme.fg_secondary)
                };
                ctx.draw_text(&item.label, Vec2::new(panel_x + 40.0, text_y), &fonts.ui, label_color);

                // Description (right-aligned, muted)
                if !item.description.is_empty() {
                    let desc_w = fonts.measure_ui(&item.description);
                    ctx.draw_text(
                        &item.description,
                        Vec2::new(panel_x + panel_w - desc_w - 16.0, text_y),
                        &fonts.ui,
                        opacity_color(theme.fg_muted),
                    );
                }
            }
        });

        ctx.canvas.restore();
    }

    fn on_event(&mut self, event: &UiEvent, _rect: Rect, _theme: &Theme) -> EventResult {
        if !self.visible && self.opacity.value() < 0.01 {
            return EventResult::Ignored;
        }
        match event {
            UiEvent::KeyDown { key, modifiers } => {
                match key {
                    Key::Escape => {
                        self.hide();
                        return EventResult::Consumed;
                    }
                    Key::Enter => {
                        self.confirm_selection();
                        return EventResult::Consumed;
                    }
                    Key::Up => {
                        self.move_selection(-1);
                        return EventResult::Consumed;
                    }
                    Key::Down => {
                        self.move_selection(1);
                        return EventResult::Consumed;
                    }
                    Key::Backspace => {
                        self.query.pop();
                        self.update_filtered();
                        return EventResult::Consumed;
                    }
                    _ => {}
                }
                EventResult::Consumed // absorb all keys while open
            }
            UiEvent::TextInput(text) if self.visible => {
                self.query.push_str(text);
                self.update_filtered();
                EventResult::Consumed
            }
            UiEvent::MouseDown { pos, button } if *button == MouseButton::Left && self.visible => {
                // Clicking outside panel closes it
                self.hide();
                EventResult::Consumed
            }
            _ => {
                if self.visible { EventResult::Consumed } else { EventResult::Ignored }
            }
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        let a = self.opacity.update(dt);
        let b = self.scale.update(dt);
        // Hide after fade-out completes
        if !a && self.opacity.value() < 0.01 {
            self.visible = false;
        }
        a || b
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}
