//! Theme system — central place for colors, fonts, and metrics.
//!
//! Everything visual is parameterized through the theme. To reskin the
//! editor, you change values here; no widget hardcodes colors.

use std::collections::HashMap;

use crate::math::{Color, Insets};

/// The complete visual theme for Moce.
pub struct Theme {
    // ── Background layers ─────────────────────────────────────────────
    /// Deepest background (the editor surface itself).
    pub bg_base: Color,
    /// Slightly raised surface (sidebars, tab bar background).
    pub bg_surface: Color,
    /// More elevated surface (popups, dropdowns).
    pub bg_elevated: Color,
    /// Hover state for interactive elements.
    pub bg_hover: Color,
    /// Active / pressed state.
    pub bg_active: Color,
    /// Title bar background.
    pub bg_title: Color,

    // ── Foreground ─────────────────────────────────────────────────────
    /// Primary text color.
    pub fg_primary: Color,
    /// Secondary text (less important labels).
    pub fg_secondary: Color,
    /// Muted text (placeholders, hints).
    pub fg_muted: Color,
    /// Text on accent-colored backgrounds.
    pub fg_on_accent: Color,

    // ── Accent ────────────────────────────────────────────────────────
    /// Main accent color (buttons, active tab indicator, selection).
    pub accent: Color,
    /// Hover state of the accent.
    pub accent_hover: Color,
    /// Dimmed accent (inactive focus, subtle highlights).
    pub accent_dim: Color,

    // ── Borders ───────────────────────────────────────────────────────
    /// Standard border color.
    pub border: Color,
    /// Subtle border (inner separators).
    pub border_subtle: Color,

    // ── Editor-specific ───────────────────────────────────────────────
    /// Selection background (unfocused).
    pub selection_bg: Color,
    /// Selection background (focused).
    pub selection_bg_focus: Color,
    /// Caret color.
    pub cursor: Color,
    /// Current line highlight.
    pub line_highlight: Color,
    /// Line number text.
    pub line_number: Color,
    /// Active line number text.
    pub line_number_active: Color,
    /// Gutter background.
    pub gutter_bg: Color,

    // ── Syntax colors ─────────────────────────────────────────────────
    pub syntax: HashMap<&'static str, Color>,

    // ── Metrics ───────────────────────────────────────────────────────
    /// Editor font size in pixels.
    pub font_size: f32,
    /// Line height multiplier (1.0 = font size).
    pub line_height: f32,
    /// Tab bar height.
    pub tab_height: f32,
    /// Status bar height.
    pub status_bar_height: f32,
    /// Title bar height.
    pub title_bar_height: f32,
    /// Scrollbar width.
    pub scrollbar_width: f32,
    /// Gutter padding.
    pub gutter_padding: f32,
    /// Default padding for UI elements.
    pub padding: Insets,
    /// Corner radius for buttons and panels.
    pub radius: f32,
}

impl Theme {
    /// A modern dark theme — the default for Moce.
    pub fn dark() -> Self {
        let mut syntax = HashMap::new();
        syntax.insert("text", Color::rgb(208, 213, 223));
        syntax.insert("keyword", Color::rgb(198, 120, 221));     // purple
        syntax.insert("control_keyword", Color::rgb(204, 120, 175)); // pink-purple
        syntax.insert("type", Color::rgb(229, 192, 120));        // yellow
        syntax.insert("function", Color::rgb(97, 175, 239));    // blue
        syntax.insert("string", Color::rgb(152, 195, 121));      // green
        syntax.insert("number", Color::rgb(209, 154, 102));      // orange
        syntax.insert("comment", Color::rgb(92, 103, 122));      // gray-blue
        syntax.insert("doc_comment", Color::rgb(103, 116, 136));
        syntax.insert("operator", Color::rgb(171, 178, 191));    // light gray
        syntax.insert("punctuation", Color::rgb(171, 178, 191));
        syntax.insert("identifier", Color::rgb(208, 213, 223));
        syntax.insert("macro", Color::rgb(86, 182, 194));        // teal
        syntax.insert("attribute", Color::rgb(209, 154, 102));
        syntax.insert("lifetime", Color::rgb(229, 192, 120));
        syntax.insert("boolean", Color::rgb(209, 154, 102));
        syntax.insert("constant", Color::rgb(209, 154, 102));

        Self {
            bg_base: Color::rgb(24, 25, 31),
            bg_surface: Color::rgb(28, 30, 37),
            bg_elevated: Color::rgb(35, 37, 45),
            bg_hover: Color::rgb(42, 44, 53),
            bg_active: Color::rgb(49, 51, 61),
            bg_title: Color::rgb(20, 21, 27),

            fg_primary: Color::rgb(208, 213, 223),
            fg_secondary: Color::rgb(145, 151, 165),
            fg_muted: Color::rgb(100, 106, 120),
            fg_on_accent: Color::rgb(255, 255, 255),

            accent: Color::rgb(97, 175, 239),
            accent_hover: Color::rgb(115, 190, 252),
            accent_dim: Color::rgba(97, 175, 239, 40),

            border: Color::rgb(45, 47, 55),
            border_subtle: Color::rgb(38, 40, 47),

            selection_bg: Color::rgb(44, 48, 59),
            selection_bg_focus: Color::rgb(43, 71, 108),
            cursor: Color::rgb(208, 213, 223),
            line_highlight: Color::rgba(255, 255, 255, 5),
            line_number: Color::rgb(85, 91, 105),
            line_number_active: Color::rgb(160, 166, 180),
            gutter_bg: Color::rgb(24, 25, 31),

            syntax,

            font_size: 15.0,
            line_height: 1.6,
            tab_height: 36.0,
            status_bar_height: 24.0,
            title_bar_height: 38.0,
            scrollbar_width: 12.0,
            gutter_padding: 16.0,
            padding: Insets::symmetrical(8.0, 4.0),
            radius: 6.0,
        }
    }

    /// A modern light theme.
    #[allow(dead_code)]
    pub fn light() -> Self {
        let mut syntax = HashMap::new();
        syntax.insert("text", Color::rgb(56, 58, 66));
        syntax.insert("keyword", Color::rgb(165, 91, 190));
        syntax.insert("control_keyword", Color::rgb(165, 91, 190));
        syntax.insert("type", Color::rgb(162, 132, 18));
        syntax.insert("function", Color::rgb(48, 123, 199));
        syntax.insert("string", Color::rgb(106, 139, 70));
        syntax.insert("number", Color::rgb(180, 110, 60));
        syntax.insert("comment", Color::rgb(140, 145, 155));
        syntax.insert("doc_comment", Color::rgb(130, 135, 145));
        syntax.insert("operator", Color::rgb(80, 82, 90));
        syntax.insert("punctuation", Color::rgb(80, 82, 90));
        syntax.insert("identifier", Color::rgb(56, 58, 66));
        syntax.insert("macro", Color::rgb(60, 160, 170));
        syntax.insert("attribute", Color::rgb(180, 110, 60));
        syntax.insert("lifetime", Color::rgb(162, 132, 18));
        syntax.insert("boolean", Color::rgb(180, 110, 60));
        syntax.insert("constant", Color::rgb(180, 110, 60));

        Self {
            bg_base: Color::rgb(252, 252, 253),
            bg_surface: Color::rgb(248, 248, 250),
            bg_elevated: Color::rgb(255, 255, 255),
            bg_hover: Color::rgb(240, 241, 243),
            bg_active: Color::rgb(230, 231, 235),
            bg_title: Color::rgb(244, 245, 247),

            fg_primary: Color::rgb(56, 58, 66),
            fg_secondary: Color::rgb(110, 115, 125),
            fg_muted: Color::rgb(150, 155, 165),
            fg_on_accent: Color::rgb(255, 255, 255),

            accent: Color::rgb(48, 123, 199),
            accent_hover: Color::rgb(65, 138, 212),
            accent_dim: Color::rgba(48, 123, 199, 30),

            border: Color::rgb(220, 222, 225),
            border_subtle: Color::rgb(230, 232, 235),

            selection_bg: Color::rgb(220, 230, 245),
            selection_bg_focus: Color::rgb(180, 210, 240),
            cursor: Color::rgb(56, 58, 66),
            line_highlight: Color::rgba(0, 0, 0, 4),
            line_number: Color::rgb(160, 165, 175),
            line_number_active: Color::rgb(80, 85, 95),
            gutter_bg: Color::rgb(248, 248, 250),

            syntax,

            font_size: 15.0,
            line_height: 1.6,
            tab_height: 36.0,
            status_bar_height: 24.0,
            title_bar_height: 38.0,
            scrollbar_width: 12.0,
            gutter_padding: 16.0,
            padding: Insets::symmetrical(8.0, 4.0),
            radius: 6.0,
        }
    }

    /// Get a syntax color by theme key. Falls back to the default text color.
    pub fn syntax_color(&self, key: &str) -> Color {
        *self.syntax.get(key).unwrap_or(&self.fg_primary)
    }

    /// Get the pixel line height (font size * line height multiplier).
    #[inline]
    pub fn line_height_px(&self) -> f32 {
        self.font_size * self.line_height
    }
}
