//! Find / Replace bar widget.
//!
//! Appears at the top of the editor area when triggered (Ctrl+F / Ctrl+H).
//! Shows a compact bar with two text fields (find + replace), match count,
//! navigation arrows, and a close button.

use crate::editor::search::SearchState;
use crate::math::{Color, Rect, Vec2};
use crate::ui::animation::Anim;
use crate::ui::widget::{EventResult, Key, Modifiers, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

/// Which field is focused in the bar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FindFocus {
    Find,
    Replace,
}

/// Events emitted by the find/replace bar.
#[derive(Debug, Clone)]
pub enum FindReplaceEvent {
    FindNext,
    FindPrev,
    ReplaceOne,
    ReplaceAll,
    Close,
    QueryChanged(String),
}

/// The find/replace bar shown at the top of the editor.
pub struct FindReplaceBar {
    pub visible: bool,
    pub show_replace: bool,
    pub query: String,
    pub replacement: String,
    pub focus: FindFocus,
    pub match_count: usize,
    pub current_match: Option<usize>,
    pub case_sensitive: bool,
    pub whole_word: bool,
    slide: Anim<f32>, // 0.0 = hidden (slide up), 1.0 = fully shown
    pub events: Vec<FindReplaceEvent>,
}

impl FindReplaceBar {
    pub fn new() -> Self {
        Self {
            visible: false,
            show_replace: false,
            query: String::new(),
            replacement: String::new(),
            focus: FindFocus::Find,
            match_count: 0,
            current_match: None,
            case_sensitive: false,
            whole_word: false,
            slide: Anim::new(0.0),
            events: Vec::new(),
        }
    }

    pub fn show(&mut self, replace: bool) {
        self.visible = true;
        self.show_replace = replace;
        self.slide.animate_to(1.0, 0.14);
        self.focus = FindFocus::Find;
    }

    pub fn hide(&mut self) {
        self.slide.animate_to(0.0, 0.1);
        self.visible = false;
    }

    pub fn bar_height(&self) -> f32 {
        let base = if self.show_replace { 80.0 } else { 42.0 };
        base * self.slide.value()
    }

    pub fn take_events(&mut self) -> Vec<FindReplaceEvent> {
        std::mem::take(&mut self.events)
    }

    fn draw_field(
        &self,
        ctx: &DrawCtx<'_>,
        rect: Rect,
        value: &str,
        placeholder: &str,
        focused: bool,
        theme: &Theme,
        fonts: &Fonts,
    ) {
        let bg = if focused { theme.bg_active } else { theme.bg_surface };
        ctx.fill_rrect(rect, 4.0, bg);
        ctx.stroke_rrect(
            rect,
            4.0,
            if focused { theme.accent } else { theme.border },
            1.0,
        );
        let text_y = rect.y + (rect.h - fonts.mono.size()) / 2.0;
        if value.is_empty() {
            ctx.draw_text_clipped(placeholder, Vec2::new(rect.x + 8.0, text_y), &fonts.ui, theme.fg_muted, rect);
        } else {
            ctx.draw_text_clipped(value, Vec2::new(rect.x + 8.0, text_y), &fonts.mono, theme.fg_primary, rect);
        }
        // Cursor
        if focused {
            let cw = fonts.measure_mono(value);
            ctx.fill_rect(
                Rect::new(rect.x + 8.0 + cw, rect.y + 6.0, 2.0, rect.h - 12.0),
                theme.cursor,
            );
        }
    }
}

impl Widget for FindReplaceBar {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, fonts: &Fonts) {
        let h = self.bar_height();
        if h < 2.0 { return; }

        let bar_rect = Rect::new(rect.x, rect.y, rect.w, h);
        ctx.fill_rect(bar_rect, theme.bg_surface);
        ctx.draw_hline(rect.x, rect.y + h - 1.0, rect.w, theme.border, 1.0);

        let pad = 8.0;
        let field_h = 26.0;
        let field_w = (rect.w * 0.35).min(300.0);
        let fy = rect.y + (42.0 - field_h) / 2.0 * self.slide.value();

        // Find field
        self.draw_field(
            ctx,
            Rect::new(rect.x + pad, fy, field_w, field_h),
            &self.query.clone(),
            "Find…",
            self.focus == FindFocus::Find,
            theme,
            fonts,
        );

        // Match count badge
        let count_x = rect.x + pad + field_w + 8.0;
        let count_text = if self.match_count == 0 && !self.query.is_empty() {
            "No results".to_string()
        } else if self.match_count > 0 {
            if let Some(cur) = self.current_match {
                format!("{}/{}", cur + 1, self.match_count)
            } else {
                format!("{} matches", self.match_count)
            }
        } else {
            String::new()
        };
        let count_color = if self.match_count == 0 && !self.query.is_empty() {
            theme.syntax_color("string") // use red-ish for "no results"
        } else {
            theme.fg_muted
        };
        ctx.draw_text(&count_text, Vec2::new(count_x, fy + (field_h - fonts.ui.size()) / 2.0), &fonts.ui, count_color);

        // ← → nav buttons
        let nav_x = count_x + fonts.measure_ui(&count_text) + 8.0;
        let nav_y = fy + (field_h - 20.0) / 2.0;
        ctx.fill_rrect(Rect::new(nav_x, nav_y, 24.0, 20.0), 4.0, theme.bg_hover);
        ctx.draw_text("↑", Vec2::new(nav_x + 7.0, nav_y + 2.0), &fonts.ui, theme.fg_secondary);
        ctx.fill_rrect(Rect::new(nav_x + 28.0, nav_y, 24.0, 20.0), 4.0, theme.bg_hover);
        ctx.draw_text("↓", Vec2::new(nav_x + 35.0, nav_y + 2.0), &fonts.ui, theme.fg_secondary);

        // Options: Aa (case) and W (word)
        let opts_x = nav_x + 60.0;
        let case_active = self.case_sensitive;
        let word_active = self.whole_word;
        ctx.fill_rrect(Rect::new(opts_x, nav_y, 24.0, 20.0), 4.0, if case_active { theme.accent_dim } else { theme.bg_hover });
        ctx.draw_text("Aa", Vec2::new(opts_x + 2.0, nav_y + 2.0), &fonts.ui, if case_active { theme.accent } else { theme.fg_muted });
        ctx.fill_rrect(Rect::new(opts_x + 28.0, nav_y, 24.0, 20.0), 4.0, if word_active { theme.accent_dim } else { theme.bg_hover });
        ctx.draw_text("W", Vec2::new(opts_x + 34.0, nav_y + 2.0), &fonts.ui, if word_active { theme.accent } else { theme.fg_muted });

        // Close button
        let close_x = rect.right() - 32.0;
        let close_y = fy + (field_h - 20.0) / 2.0;
        ctx.fill_rrect(Rect::new(close_x, close_y, 20.0, 20.0), 4.0, theme.bg_hover);
        let cx = close_x + 10.0;
        let cy = close_y + 10.0;
        let r = 4.0;
        ctx.draw_line(Vec2::new(cx - r, cy - r), Vec2::new(cx + r, cy + r), theme.fg_muted, 1.5);
        ctx.draw_line(Vec2::new(cx + r, cy - r), Vec2::new(cx - r, cy + r), theme.fg_muted, 1.5);

        // Replace field (if visible)
        if self.show_replace && h > 55.0 {
            let ry = fy + 38.0;
            self.draw_field(
                ctx,
                Rect::new(rect.x + pad, ry, field_w, field_h),
                &self.replacement.clone(),
                "Replace…",
                self.focus == FindFocus::Replace,
                theme,
                fonts,
            );
            let repl_nav_x = rect.x + pad + field_w + 8.0;
            let repl_nav_y = ry + (field_h - 20.0) / 2.0;
            // Replace one
            ctx.fill_rrect(Rect::new(repl_nav_x, repl_nav_y, 70.0, 20.0), 4.0, theme.bg_hover);
            ctx.draw_text("Replace", Vec2::new(repl_nav_x + 4.0, repl_nav_y + 2.0), &fonts.ui, theme.fg_secondary);
            // Replace all
            ctx.fill_rrect(Rect::new(repl_nav_x + 76.0, repl_nav_y, 80.0, 20.0), 4.0, theme.bg_hover);
            ctx.draw_text("Replace All", Vec2::new(repl_nav_x + 80.0, repl_nav_y + 2.0), &fonts.ui, theme.fg_secondary);
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _theme: &Theme) -> EventResult {
        if !self.visible { return EventResult::Ignored; }
        let h = self.bar_height();
        if h < 2.0 { return EventResult::Ignored; }
        let bar_rect = Rect::new(rect.x, rect.y, rect.w, h);

        match event {
            UiEvent::KeyDown { key, modifiers } => {
                match key {
                    Key::Escape => {
                        self.events.push(FindReplaceEvent::Close);
                        self.hide();
                        return EventResult::Consumed;
                    }
                    Key::Enter => {
                        if modifiers.shift {
                            self.events.push(FindReplaceEvent::FindPrev);
                        } else {
                            self.events.push(FindReplaceEvent::FindNext);
                        }
                        return EventResult::Consumed;
                    }
                    Key::Tab => {
                        // Toggle focus between find and replace
                        self.focus = match self.focus {
                            FindFocus::Find => FindFocus::Replace,
                            FindFocus::Replace => FindFocus::Find,
                        };
                        return EventResult::Consumed;
                    }
                    Key::Backspace => {
                        match self.focus {
                            FindFocus::Find => { self.query.pop(); self.events.push(FindReplaceEvent::QueryChanged(self.query.clone())); }
                            FindFocus::Replace => { self.replacement.pop(); }
                        }
                        return EventResult::Consumed;
                    }
                    _ => {}
                }
                EventResult::Consumed
            }
            UiEvent::TextInput(text) => {
                match self.focus {
                    FindFocus::Find => {
                        self.query.push_str(text);
                        self.events.push(FindReplaceEvent::QueryChanged(self.query.clone()));
                    }
                    FindFocus::Replace => { self.replacement.push_str(text); }
                }
                EventResult::Consumed
            }
            UiEvent::MouseDown { pos, button } if *button == MouseButton::Left => {
                if bar_rect.contains(*pos) {
                    // Check close button
                    let close_x = rect.right() - 32.0;
                    if pos.x >= close_x {
                        self.events.push(FindReplaceEvent::Close);
                        self.hide();
                    }
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }
            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        self.slide.update(dt)
    }
}

impl Default for FindReplaceBar {
    fn default() -> Self {
        Self::new()
    }
}
