//! The main editor view widget — renders text with syntax highlighting,
//! cursor, selection, line numbers, and handles keyboard/mouse input.

use crate::editor::document::CursorDirection;
use crate::editor::search::SearchState;
use crate::editor::Document;
use crate::math::{Color, Rect, Vec2};
use crate::syntax::Highlighter;
use crate::ui::animation::Anim;
use crate::ui::widget::{EventResult, Key, Modifiers, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};
use crate::app::moce_app::ClipboardOp;

pub struct EditorView {
    pub doc: Document,
    highlighter: Highlighter,
    cursor_blink: Anim<f32>,
    blink_timer: f32,
    focused: bool,
    scroll_y_anim: Anim<f32>,
    scroll_x_anim: Anim<f32>,
    gutter_width: f32,
    mouse_selecting: bool,
    /// Active search state.
    search: SearchState,
    /// Pending clipboard operation to be handled by the app.
    pending_clipboard: Option<ClipboardOp>,
}

impl EditorView {
    pub fn new(doc: Document) -> Self {
        let lang = doc.language;
        Self {
            doc,
            highlighter: Highlighter::new(lang),
            cursor_blink: Anim::new(1.0),
            blink_timer: 0.0,
            focused: true,
            scroll_y_anim: Anim::new(0.0),
            scroll_x_anim: Anim::new(0.0),
            gutter_width: 0.0,
            mouse_selecting: false,
            search: SearchState::new(),
            pending_clipboard: None,
        }
    }

    // ── Public API for the app layer ───────────────────────────────────

    /// Current vertical scroll offset in pixels.
    pub fn scroll_y(&self) -> f32 {
        self.scroll_y_anim.value()
    }

    /// Mutable access to the syntax highlighter (for the minimap).
    pub fn highlighter_mut(&mut self) -> &mut Highlighter {
        &mut self.highlighter
    }

    /// Set the search query and update matches.
    pub fn set_search_query(&mut self, query: String) {
        self.search.query = query;
        self.search.update_matches(&self.doc.buffer);
    }

    /// Number of search matches in the current document.
    pub fn search_match_count(&self) -> usize {
        self.search.matches.len()
    }

    /// Move to the next search match.
    pub fn find_next(&mut self) {
        if let Some(m) = self.search.next_match(self.doc.cursor.position) {
            self.doc.cursor.move_to(m.start, false);
        }
    }

    /// Move to the previous search match.
    pub fn find_prev(&mut self) {
        if let Some(m) = self.search.prev_match(self.doc.cursor.position) {
            self.doc.cursor.move_to(m.start, false);
        }
    }

    /// Replace the current match with `replacement`.
    pub fn replace_current(&mut self, replacement: &str) {
        if let Some(idx) = self.search.current_match {
            if let Some(m) = self.search.matches.get(idx) {
                self.doc.delete_range(m.start, m.end);
                self.doc.insert_text(replacement);
                self.search.update_matches(&self.doc.buffer);
            }
        }
    }

    /// Replace all matches with `replacement`. Returns the count.
    pub fn replace_all(&mut self, replacement: &str) -> usize {
        self.search.update_matches(&self.doc.buffer);
        let count = self.search.matches.len();
        // Apply from last to first to keep offsets valid
        for m in self.search.matches.iter().rev().cloned().collect::<Vec<_>>() {
            self.doc.delete_range(m.start, m.end);
            self.doc.insert_text(replacement);
        }
        self.search.update_matches(&self.doc.buffer);
        count
    }

    /// Take any pending clipboard operation (copy/cut/paste request).
    pub fn take_clipboard_op(&mut self) -> Option<ClipboardOp> {
        self.pending_clipboard.take()
    }

    /// Convert a pixel position (relative to the editor's top-left) to a
    /// (line, col) position in the document.
    fn pixel_to_position(
        &self,
        pos: Vec2,
        gutter_width: f32,
        line_height: f32,
        char_width: f32,
        scroll_y: f32,
        scroll_x: f32,
    ) -> crate::editor::Position {
        use crate::editor::Position;
        let content_x = pos.x - gutter_width;
        let content_y = pos.y + scroll_y;

        let line = ((content_y / line_height) as isize).max(0) as usize;
        let line = line.min(self.doc.buffer.len_lines().saturating_sub(1));

        let col_f = (content_x + scroll_x) / char_width;
        let col = (col_f.max(0.0) as usize).min(self.doc.buffer.line_len(line));

        Position::new(line, col)
    }

    /// Scroll the view so the cursor is visible.
    fn scroll_to_cursor(&mut self, line_height: f32, viewport_h: f32, viewport_w: f32, gutter_w: f32, char_w: f32) {
        let cur = self.doc.cursor.position;

        // Vertical
        let cursor_top = cur.line as f32 * line_height;
        let cursor_bottom = cursor_top + line_height;
        let scroll_y = self.scroll_y_anim.value();
        let margin = line_height * 3.0;

        let new_scroll_y = if cursor_top < scroll_y + margin {
            (cursor_top - margin).max(0.0)
        } else if cursor_bottom > scroll_y + viewport_h - margin {
            cursor_bottom - viewport_h + margin
        } else {
            scroll_y
        };

        if (new_scroll_y - scroll_y).abs() > 0.5 {
            self.scroll_y_anim.animate_to(new_scroll_y, 0.12);
        }

        // Horizontal
        let cursor_x = cur.col as f32 * char_w;
        let scroll_x = self.scroll_x_anim.value();
        let h_margin = char_w * 5.0;

        let new_scroll_x = if cursor_x < scroll_x + h_margin {
            (cursor_x - h_margin).max(0.0)
        } else if cursor_x > scroll_x + (viewport_w - gutter_w) - h_margin {
            cursor_x - (viewport_w - gutter_w) + h_margin
        } else {
            scroll_x
        };

        if (new_scroll_x - scroll_x).abs() > 0.5 {
            self.scroll_x_anim.animate_to(new_scroll_x, 0.12);
        }
    }
}

impl Widget for EditorView {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, fonts: &Fonts) {
        let line_height = theme.line_height_px();
        let char_w = fonts.char_width_mono();
        let total_lines = self.doc.buffer.len_lines();

        // Compute gutter width based on number of digits in line count
        let digits = total_lines.to_string().len().max(2);
        let gutter_w = (digits as f32 * char_w + theme.gutter_padding * 2.0).max(48.0);
        self.gutter_width = gutter_w;

        let scroll_y = self.scroll_y_anim.value();
        let scroll_x = self.scroll_x_anim.value();

        let content_rect = Rect::new(
            rect.x + gutter_w,
            rect.y,
            rect.w - gutter_w,
            rect.h,
        );

        // ── Background ────────────────────────────────────────────────
        ctx.fill_rect(rect, theme.bg_base);
        ctx.fill_rect(Rect::new(rect.x, rect.y, gutter_w, rect.h), theme.gutter_bg);
        ctx.draw_vline(rect.x + gutter_w - 1.0, rect.y, rect.h, theme.border_subtle, 1.0);

        // Determine visible line range
        let first_line = (scroll_y / line_height) as usize;
        let last_line = ((scroll_y + rect.h) / line_height) as usize + 1;
        let last_line = last_line.min(total_lines.saturating_sub(1));

        let cur_line = self.doc.cursor.position.line;
        let selection = self.doc.cursor.selection_range();

        // ── Render lines ──────────────────────────────────────────────
        ctx.clip(content_rect, |ctx| {
            for line_idx in first_line..=last_line {
                let y = rect.y + line_idx as f32 * line_height - scroll_y;

                // Current-line highlight
                if line_idx == cur_line && self.focused {
                    ctx.fill_rect(
                        Rect::new(content_rect.x, y, content_rect.w, line_height),
                        theme.line_highlight,
                    );
                }

                let line_text = self.doc.buffer.line(line_idx);
                if line_text.is_empty() {
                    continue;
                }

                // ── Search match highlight ────────────────────────────────
                if !self.search.query.is_empty() {
                    for m in &self.search.matches {
                        if m.start.line == line_idx {
                            let mx = content_rect.x + m.start.col as f32 * char_w - scroll_x;
                            let mw = (m.end.col - m.start.col) as f32 * char_w;
                            let highlight = if self.search.is_focused_match_at(line_idx, m.start.col) {
                                theme.syntax_color("number") // focused match = orange
                            } else {
                                Color::rgba(255, 200, 80, 60) // other matches = dim yellow
                            };
                            ctx.fill_rect(Rect::new(mx, y, mw, line_height), highlight);
                        }
                    }
                }

                // ── Selection highlight ────────────────────────────────
                if let Some((sel_start, sel_end)) = selection {
                    use crate::editor::Position;
                    let line_start = Position::new(line_idx, 0);
                    let line_end = Position::new(line_idx, self.doc.buffer.line_len(line_idx));

                    if sel_start <= line_end && sel_end >= line_start {
                        let sel_col_start = if sel_start.line < line_idx {
                            0
                        } else {
                            sel_start.col
                        };
                        let sel_col_end = if sel_end.line > line_idx {
                            self.doc.buffer.line_len(line_idx)
                        } else {
                            sel_end.col
                        };
                        let sel_x = content_rect.x + sel_col_start as f32 * char_w - scroll_x;
                        let sel_w = (sel_col_end - sel_col_start) as f32 * char_w;
                        if sel_w > 0.0 {
                            let sel_color = if self.focused {
                                theme.selection_bg_focus
                            } else {
                                theme.selection_bg
                            };
                            ctx.fill_rect(
                                Rect::new(sel_x, y, sel_w, line_height),
                                sel_color,
                            );
                        }
                    }
                }

                // ── Syntax-highlighted text ────────────────────────────
                let tokens = self.highlighter.tokenize_line(&line_text);
                for token in &tokens {
                    let start = token.start.min(line_text.len());
                    let end = token.end.min(line_text.len());
                    if start >= end {
                        continue;
                    }
                    let slice = &line_text[start..end];
                    let col_offset = line_text[..start].chars().count();
                    let x = content_rect.x + col_offset as f32 * char_w - scroll_x;
                    let text_y = y + (line_height - fonts.mono.size()) / 2.0;

                    let color = theme.syntax_color(token.kind.theme_key());
                    ctx.draw_text(slice, Vec2::new(x, text_y), &fonts.mono, color);
                }
            }
        });

        // ── Gutter (line numbers) ─────────────────────────────────────
        ctx.clip(Rect::new(rect.x, rect.y, gutter_w, rect.h), |ctx| {
            for line_idx in first_line..=last_line {
                let y = rect.y + line_idx as f32 * line_height - scroll_y;
                let display_num = (line_idx + 1).to_string();
                let num_w = fonts.measure_mono(&display_num);
                let num_x = rect.x + gutter_w - num_w - theme.gutter_padding;
                let num_y = y + (line_height - fonts.mono.size()) / 2.0;
                let color = if line_idx == cur_line {
                    theme.line_number_active
                } else {
                    theme.line_number
                };
                ctx.draw_text(&display_num, Vec2::new(num_x, num_y), &fonts.mono, color);
            }
        });

        // ── Cursor ────────────────────────────────────────────────────
        let blink = self.cursor_blink.value();
        if blink > 0.1 && self.focused {
            let cur = self.doc.cursor.position;
            let cur_x = content_rect.x + cur.col as f32 * char_w - scroll_x;
            let cur_y = rect.y + cur.line as f32 * line_height - scroll_y;
            // Block cursor: draw as a vertical bar (I-beam style)
            let cursor_w = 2.0;
            ctx.fill_rect(
                Rect::new(cur_x, cur_y + 2.0, cursor_w, line_height - 4.0),
                theme.cursor,
            );
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, theme: &Theme) -> EventResult {
        let line_height = theme.line_height_px();
        // Use approximate char width since fonts aren't available here.
        // The actual char width is computed during paint; here we use a fixed estimate.
        let char_w = theme.font_size * 0.6;
        let gutter_w = self.gutter_width.max(48.0);
        let scroll_y = self.scroll_y_anim.value();
        let scroll_x = self.scroll_x_anim.value();
        let viewport_h = rect.h;
        let viewport_w = rect.w;

        match event {
            UiEvent::MouseDown { pos, button } if *button == MouseButton::Left => {
                if rect.contains(*pos) {
                    self.focused = true;
                    let rel = Vec2::new(pos.x - rect.x, pos.y - rect.y);
                    let position = self.pixel_to_position(rel, gutter_w, line_height, char_w, scroll_y, scroll_x);
                    self.doc.cursor.move_to(position, false);
                    self.mouse_selecting = true;
                    // Reset blink timer on click
                    self.blink_timer = 0.0;
                    self.cursor_blink.jump_to(1.0);
                    return EventResult::Consumed;
                } else {
                    self.focused = false;
                }
                EventResult::Ignored
            }
            UiEvent::MouseMove { pos } => {
                if self.mouse_selecting {
                    let rel = Vec2::new(pos.x - rect.x, pos.y - rect.y);
                    let position = self.pixel_to_position(rel, gutter_w, line_height, char_w, scroll_y, scroll_x);
                    self.doc.cursor.move_to(position, true);
                    return EventResult::Consumed;
                }
                EventResult::Ignored
            }
            UiEvent::MouseUp { button, .. } if *button == MouseButton::Left => {
                self.mouse_selecting = false;
                EventResult::Ignored
            }
            UiEvent::Scroll { delta, pos } if rect.contains(*pos) => {
                // Smooth scroll
                let new_y = (self.scroll_y_anim.value() - delta.y * line_height * 3.0).max(0.0);
                let max_y = (self.doc.buffer.len_lines() as f32 * line_height - viewport_h).max(0.0);
                let new_y = new_y.min(max_y);
                self.scroll_y_anim.animate_to(new_y, 0.08);
                return EventResult::Consumed;
            }
            UiEvent::KeyDown { key, modifiers } if self.focused => {
                self.blink_timer = 0.0;
                self.cursor_blink.jump_to(1.0);

                let extend = modifiers.shift;

                match key {
                    Key::Left => {
                        if modifiers.ctrl {
                            self.doc.move_cursor(CursorDirection::WordLeft, extend);
                        } else {
                            self.doc.move_cursor(CursorDirection::Left, extend);
                        }
                    }
                    Key::Right => {
                        if modifiers.ctrl {
                            self.doc.move_cursor(CursorDirection::WordRight, extend);
                        } else {
                            self.doc.move_cursor(CursorDirection::Right, extend);
                        }
                    }
                    Key::Up => self.doc.move_cursor(CursorDirection::Up, extend),
                    Key::Down => self.doc.move_cursor(CursorDirection::Down, extend),
                    Key::Home => {
                        if modifiers.ctrl {
                            self.doc.move_cursor(CursorDirection::DocStart, extend);
                        } else {
                            self.doc.move_cursor(CursorDirection::Home, extend);
                        }
                    }
                    Key::End => {
                        if modifiers.ctrl {
                            self.doc.move_cursor(CursorDirection::DocEnd, extend);
                        } else {
                            self.doc.move_cursor(CursorDirection::End, extend);
                        }
                    }
                    Key::Backspace => {
                        if modifiers.ctrl {
                            self.doc.delete_word_back();
                        } else {
                            self.doc.backspace();
                        }
                    }
                    Key::Delete => self.doc.delete_forward(),
                    Key::Enter => self.doc.insert_newline(),
                    Key::Tab => {
                        if modifiers.shift {
                            // De-indent: remove one indent unit from line start
                            // (simplified: just insert spaces for now)
                        } else {
                            self.doc.insert_text(self.doc.language.indent_unit());
                        }
                    }
                    Key::Char(c) if modifiers.ctrl => {
                        match c {
                            'a' | 'A' => self.doc.select_all(),
                            'c' | 'C' => { self.pending_clipboard = Some(ClipboardOp::Copy); }
                            'v' | 'V' => { self.pending_clipboard = Some(ClipboardOp::Paste); }
                            'x' | 'X' => { self.pending_clipboard = Some(ClipboardOp::Cut); }
                            's' | 'S' => { let _ = self.doc.save(); }
                            'z' | 'Z' => {
                                if modifiers.shift {
                                    self.doc.redo();
                                } else {
                                    self.doc.undo();
                                }
                            }
                            'y' | 'Y' => { self.doc.redo(); }
                            _ => {}
                        }
                    }
                    _ => return EventResult::Ignored,
                }

                self.scroll_to_cursor(line_height, viewport_h, viewport_w, gutter_w, char_w);
                EventResult::Consumed
            }
            UiEvent::TextInput(text) if self.focused => {
                self.doc.insert_text(text);
                self.blink_timer = 0.0;
                self.cursor_blink.jump_to(1.0);
                self.scroll_to_cursor(line_height, viewport_h, viewport_w, gutter_w, char_w);
                EventResult::Consumed
            }
            UiEvent::FocusGained => {
                self.focused = true;
                EventResult::Ignored
            }
            UiEvent::FocusLost => {
                self.focused = false;
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        // Cursor blink: 530ms on, 530ms off
        let blink_period = 0.53;
        self.blink_timer += dt;
        if self.blink_timer >= blink_period {
            self.blink_timer -= blink_period;
            let current = self.cursor_blink.value();
            let target = if current > 0.5 { 0.0 } else { 1.0 };
            self.cursor_blink.animate_to(target, 0.06);
        }

        let a = self.cursor_blink.update(dt);
        let b = self.scroll_y_anim.update(dt);
        let c = self.scroll_x_anim.update(dt);
        a || b || c
    }
}
