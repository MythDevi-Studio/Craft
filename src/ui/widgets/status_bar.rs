//! Status bar widget — shows cursor position, language, encoding, etc.

use crate::math::{Rect, Vec2};
use crate::ui::widget::{EventResult, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

/// A section of text in the status bar, with an optional click action.
#[derive(Clone)]
pub struct StatusSection {
    pub text: String,
    pub tooltip: String,
    pub right_aligned: bool,
}

impl StatusSection {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tooltip: String::new(),
            right_aligned: false,
        }
    }

    pub fn right(mut self) -> Self {
        self.right_aligned = true;
        self
    }
}

/// The status bar at the bottom of the editor.
pub struct StatusBar {
    pub sections: Vec<StatusSection>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    pub fn set_cursor(&mut self, line: usize, col: usize) {
        // Find or update the cursor section
        if let Some(s) = self.sections.iter_mut().find(|s| s.text.starts_with("Ln")) {
            s.text = format!("Ln {}, Col {}", line + 1, col + 1);
        } else {
            self.sections.push(StatusSection::new(format!("Ln {}, Col {}", line + 1, col + 1)));
        }
    }

    pub fn set_language(&mut self, lang: &str) {
        if let Some(s) = self.sections.iter_mut().find(|s| s.right_aligned) {
            s.text = lang.to_string();
        } else {
            self.sections.push(StatusSection::new(lang).right());
        }
    }

    pub fn set_message(&mut self, msg: impl Into<String>) {
        // A temporary message shown in the center
        let m = msg.into();
        // Find or create a center section
        if let Some(s) = self.sections.iter_mut().find(|s| !s.right_aligned && !s.text.starts_with("Ln")) {
            s.text = m;
        } else {
            self.sections.insert(0, StatusSection::new(m));
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for StatusBar {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, fonts: &Fonts) {
        // Background
        ctx.fill_rect(rect, theme.accent.adjust_lightness(-0.25));

        // Top border
        ctx.draw_hline(rect.x, rect.y, rect.w, theme.border, 1.0);

        let pad_x = 8.0;
        let text_y = rect.y + (rect.h - fonts.ui.size()) / 2.0;
        let text_color = theme.fg_on_accent;

        let mut left_x = rect.x + pad_x;
        let mut right_x = rect.right() - pad_x;

        for section in &self.sections {
            let tw = fonts.measure_ui(&section.text);
            if section.right_aligned {
                right_x -= tw;
                ctx.draw_text(&section.text, Vec2::new(right_x, text_y), &fonts.ui, text_color);
                right_x -= pad_x;
            } else {
                ctx.draw_text(&section.text, Vec2::new(left_x, text_y), &fonts.ui, text_color);
                left_x += tw + pad_x * 2.0;
            }
        }
    }

    fn on_event(&mut self, _event: &UiEvent, _rect: Rect, _theme: &Theme) -> EventResult {
        EventResult::Ignored
    }
}
