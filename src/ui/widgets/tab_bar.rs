//! Tab bar widget — shows open files and allows switching between them.
//!
//! Each tab shows the file name and a close button. The active tab is
//! indicated by a colored underline. Tabs scroll horizontally if there are
//! too many to fit.

use crate::math::{Color, Insets, Lerp, Rect, Size, Vec2};
use crate::ui::animation::Anim;
use crate::ui::widget::{EventResult, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

/// A single tab.
#[derive(Clone)]
pub struct Tab {
    pub id: usize,
    pub title: String,
    pub dirty: bool,
    hover: bool,
    close_hover: bool,
    hover_anim: Anim<f32>,
}

impl Tab {
    pub fn new(id: usize, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            dirty: false,
            hover: false,
            close_hover: false,
            hover_anim: Anim::new(0.0),
        }
    }

    /// Measure the preferred width of this tab.
    pub fn preferred_width(&self, fonts: &Fonts) -> f32 {
        let text_w = fonts.measure_ui(&self.title);
        text_w + 32.0 + 20.0 // padding + close button
    }
}

/// An event emitted by the tab bar.
#[derive(Debug, Clone)]
pub enum TabBarEvent {
    TabSelected(usize),
    TabClosed(usize),
    NewTabRequested,
}

/// The tab bar widget.
pub struct TabBar {
    pub tabs: Vec<Tab>,
    pub active_id: Option<usize>,
    /// Horizontal scroll offset (in pixels).
    pub scroll_x: f32,
    /// Target scroll (animated).
    scroll_anim: Anim<f32>,
    /// Events generated during this frame.
    pub events: Vec<TabBarEvent>,
    /// Active tab indicator animation (x position).
    indicator_anim: Anim<f32>,
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_id: None,
            scroll_x: 0.0,
            scroll_anim: Anim::new(0.0),
            events: Vec::new(),
            indicator_anim: Anim::new(0.0),
        }
    }

    pub fn add_tab(&mut self, tab: Tab) {
        self.tabs.push(tab);
    }

    pub fn close_tab(&mut self, id: usize) {
        self.tabs.retain(|t| t.id != id);
        if self.active_id == Some(id) {
            self.active_id = self.tabs.last().map(|t| t.id);
        }
    }

    pub fn set_active(&mut self, id: usize) {
        self.active_id = Some(id);
    }

    /// Layout: compute the rect of a specific tab given the bar width.
    fn tab_rects(&self, bar: Rect, fonts: &Fonts) -> Vec<(usize, Rect)> {
        let mut rects = Vec::new();
        let mut x = bar.x - self.scroll_anim.value();
        let h = bar.h;
        for tab in &self.tabs {
            let w = tab.preferred_width(fonts).max(80.0).min(220.0);
            rects.push((tab.id, Rect::new(x, bar.y, w, h)));
            x += w;
        }
        rects
    }

    fn close_button_rect(tab_rect: Rect) -> Rect {
        let size = 16.0;
        Rect::new(
            tab_rect.right() - size - 6.0,
            tab_rect.y + (tab_rect.h - size) / 2.0,
            size,
            size,
        )
    }

    pub fn take_events(&mut self) -> Vec<TabBarEvent> {
        std::mem::take(&mut self.events)
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for TabBar {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, fonts: &Fonts) {
        // Background
        ctx.fill_rect(rect, theme.bg_surface);
        // Bottom border
        ctx.draw_hline(rect.x, rect.bottom() - 1.0, rect.w, theme.border, 1.0);

        let scroll_offset = self.scroll_anim.value();

        // Clip tab content
        ctx.clip(rect, |ctx| {
            let tab_rects = self.tab_rects(rect, fonts);

            for (id, tab_rect) in &tab_rects {
                // Find the tab
                let tab = match self.tabs.iter().find(|t| t.id == *id) {
                    Some(t) => t,
                    None => continue,
                };
                let is_active = Some(*id) == self.active_id;
                let hover_t = tab.hover_anim.value();

                // Tab background
                let bg = if is_active {
                    theme.bg_base
                } else {
                    theme.bg_surface.lerp(&theme.bg_hover, hover_t)
                };
                ctx.fill_rect(*tab_rect, bg);

                // Right separator
                ctx.draw_vline(
                    tab_rect.right() - 1.0,
                    tab_rect.y,
                    tab_rect.h,
                    theme.border,
                    1.0,
                );

                // Active indicator bar at bottom
                if is_active {
                    let ind_h = 2.0;
                    ctx.fill_rect(
                        Rect::new(tab_rect.x, tab_rect.bottom() - ind_h, tab_rect.w, ind_h),
                        theme.accent,
                    );
                }

                // Dirty dot
                let title = if tab.dirty {
                    format!("● {}", tab.title)
                } else {
                    tab.title.clone()
                };

                // Title text
                let close_w = 20.0;
                let padding_x = 12.0;
                let available_w = tab_rect.w - close_w - padding_x * 2.0;
                let text_color = if is_active {
                    theme.fg_primary
                } else {
                    theme.fg_secondary.lerp(&theme.fg_primary, hover_t)
                };
                let text_y = tab_rect.y + (tab_rect.h - fonts.ui.size()) / 2.0;
                ctx.draw_text_clipped(
                    &title,
                    Vec2::new(tab_rect.x + padding_x, text_y),
                    &fonts.ui,
                    text_color,
                    Rect::new(tab_rect.x, tab_rect.y, tab_rect.w - close_w - 4.0, tab_rect.h),
                );

                // Close button
                let close_rect = Self::close_button_rect(*tab_rect);
                let close_color = if tab.close_hover {
                    theme.fg_primary
                } else {
                    theme.fg_muted
                };
                // Draw × glyph
                let cx = close_rect.center();
                let r = 4.5;
                ctx.draw_line(
                    Vec2::new(cx.x - r, cx.y - r),
                    Vec2::new(cx.x + r, cx.y + r),
                    close_color,
                    1.5,
                );
                ctx.draw_line(
                    Vec2::new(cx.x + r, cx.y - r),
                    Vec2::new(cx.x - r, cx.y + r),
                    close_color,
                    1.5,
                );
            }
        });

        // "+" new tab button on the right
        let new_tab_rect = Rect::new(rect.right() - 32.0, rect.y + (rect.h - 24.0) / 2.0, 24.0, 24.0);
        ctx.fill_rrect(new_tab_rect, 4.0, theme.bg_hover);
        let nc = new_tab_rect.center();
        let nr = 5.0;
        ctx.draw_hline(nc.x - nr, nc.y, nr * 2.0, theme.fg_secondary, 1.5);
        ctx.draw_vline(nc.x, nc.y - nr, nr * 2.0, theme.fg_secondary, 1.5);
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _theme: &Theme) -> EventResult {
        // We need fonts here but don't have them — approximate with fixed width
        let tab_w_estimate = 160.0_f32;

        match event {
            UiEvent::MouseMove { pos } => {
                for (idx, tab) in self.tabs.iter_mut().enumerate() {
                    // approximate rect
                    let tab_rect = Rect::new(rect.x + idx as f32 * tab_w_estimate - self.scroll_anim.value(), rect.y, tab_w_estimate, rect.h);
                    let now_hover = tab_rect.contains(*pos);
                    if now_hover != tab.hover {
                        tab.hover = now_hover;
                        if now_hover {
                            tab.hover_anim.animate_to(1.0, 0.1);
                        } else {
                            tab.hover_anim.animate_to(0.0, 0.15);
                        }
                    }
                    let close_rect = Self::close_button_rect(tab_rect);
                    tab.close_hover = close_rect.contains(*pos);
                }
                EventResult::Ignored
            }
            UiEvent::MouseDown { pos, button } if *button == MouseButton::Left => {
                // Check "new tab" button
                let new_tab_rect = Rect::new(rect.right() - 32.0, rect.y, 32.0, rect.h);
                if new_tab_rect.contains(*pos) {
                    self.events.push(TabBarEvent::NewTabRequested);
                    return EventResult::Consumed;
                }
                // Check each tab
                for (idx, tab) in self.tabs.iter().enumerate() {
                    let tab_rect = Rect::new(rect.x + idx as f32 * tab_w_estimate - self.scroll_anim.value(), rect.y, tab_w_estimate, rect.h);
                    if tab_rect.contains(*pos) {
                        let close_rect = Self::close_button_rect(tab_rect);
                        if close_rect.contains(*pos) {
                            self.events.push(TabBarEvent::TabClosed(tab.id));
                        } else {
                            self.events.push(TabBarEvent::TabSelected(tab.id));
                        }
                        return EventResult::Consumed;
                    }
                }
                EventResult::Ignored
            }
            UiEvent::Scroll { delta, pos } if rect.contains(*pos) => {
                let new_scroll = (self.scroll_anim.value() - delta.x * 40.0).max(0.0);
                self.scroll_anim.animate_to(new_scroll, 0.15);
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        let mut any = self.scroll_anim.update(dt);
        for tab in &mut self.tabs {
            if tab.hover_anim.update(dt) {
                any = true;
            }
        }
        any
    }
}
