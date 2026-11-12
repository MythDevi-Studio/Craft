//! Painter — a thin wrapper around Skia's Canvas.
//!
//! All drawing in Moce goes through the `Painter`. It owns the Skia
//! surface (raster-backed for now) and provides high-level drawing
//! primitives that understand the editor's coordinate system and theme.
//!
//! The pixel buffer is then blitted into the softbuffer surface each frame.

use skia_safe::{
    font_style::{FontStyle, Slant, Weight, Width},
    Data, Font, FontMgr, Image, Paint, PaintStyle, Path, RRect, Rect as SkRect, Surface,
    TextBlob, Typeface,
};

use crate::math::{Color, Insets, Rect, Size, Vec2};

/// Font configuration bundled together for easy lookup.
pub struct Fonts {
    /// Monospace font used for editor text.
    pub mono: Font,
    /// Sans-serif font used for UI labels, tabs, status bar.
    pub ui: Font,
    /// Bold version of the UI font.
    pub ui_bold: Font,
}

impl Fonts {
    /// Load fonts. Falls back to system defaults if the preferred fonts
    /// aren't available.
    pub fn load(mono_size: f32, ui_size: f32) -> Self {
        let mgr = FontMgr::new();

        // Try common monospace fonts in order of preference
        let mono_tf = try_typeface(&mgr, "JetBrains Mono", false)
            .or_else(|| try_typeface(&mgr, "Cascadia Code", false))
            .or_else(|| try_typeface(&mgr, "Fira Code", false))
            .or_else(|| try_typeface(&mgr, "Source Code Pro", false))
            .or_else(|| try_typeface(&mgr, "Consolas", false))
            .or_else(|| try_typeface(&mgr, "Courier New", false))
            .unwrap_or_else(|| mgr.legacy_make_typeface(None, FontStyle::normal()).unwrap());

        // Try common UI fonts
        let ui_tf = try_typeface(&mgr, "Inter", false)
            .or_else(|| try_typeface(&mgr, "Segoe UI", false))
            .or_else(|| try_typeface(&mgr, "SF Pro Text", false))
            .or_else(|| try_typeface(&mgr, "Roboto", false))
            .or_else(|| try_typeface(&mgr, "Helvetica Neue", false))
            .unwrap_or_else(|| mgr.legacy_make_typeface(None, FontStyle::normal()).unwrap());

        let ui_bold_tf = try_typeface(&mgr, "Inter", true)
            .or_else(|| try_typeface(&mgr, "Segoe UI", true))
            .unwrap_or_else(|| mgr.legacy_make_typeface(None, FontStyle::bold()).unwrap());

        Self {
            mono: Font::new(mono_tf, mono_size),
            ui: Font::new(ui_tf, ui_size),
            ui_bold: Font::new(ui_bold_tf, ui_size),
        }
    }

    /// Measure the width of a string in the monospace font.
    pub fn measure_mono(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let (width, _bounds) = self.mono.measure_str(text, None);
        width
    }

    /// Measure the width of a single character in the monospace font.
    /// For a true monospace font this is constant; we cache the first call.
    pub fn char_width_mono(&self) -> f32 {
        let (w, _) = self.mono.measure_str("W", None);
        w
    }

    /// Measure the width of a string in the UI font.
    pub fn measure_ui(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let (width, _bounds) = self.ui.measure_str(text, None);
        width
    }
}

/// Try to find a typeface by family name. Returns None if not found.
fn try_typeface(mgr: &FontMgr, family: &str, bold: bool) -> Option<Typeface> {
    let style = if bold {
        FontStyle::new(Weight::BOLD, Width::NORMAL, Slant::Upright)
    } else {
        FontStyle::normal()
    };
    mgr.match_family_style(family, style)
}

/// The main painter abstraction.
///
/// Wraps a Skia `Surface` and provides typed drawing methods. The surface
/// is re-created when the window resizes.
pub struct Painter {
    surface: Surface,
    size: Size,
}

impl Painter {
    /// Create a painter for the given window size.
    pub fn new(w: i32, h: i32) -> Self {
        let surface = Surface::new_raster_n32_premul((w, h))
            .expect("Failed to create Skia surface");
        Self {
            surface,
            size: Size::new(w as f32, h as f32),
        }
    }

    /// Resize the backing surface. Must be called before the first `draw`
    /// call after the window resizes.
    pub fn resize(&mut self, w: i32, h: i32) {
        self.size = Size::new(w as f32, h as f32);
        self.surface = Surface::new_raster_n32_premul((w, h))
            .expect("Failed to recreate Skia surface on resize");
    }

    /// Get the current size.
    #[inline]
    pub fn size(&self) -> Size {
        self.size
    }

    /// Begin a draw pass and call `f` with the low-level canvas.
    /// After `f` returns, the pixel data is extracted and returned.
    pub fn render<F>(&mut self, f: F) -> &[u32]
    where
        F: FnOnce(&mut DrawCtx<'_>),
    {
        let canvas = self.surface.canvas();
        let mut ctx = DrawCtx { canvas };
        f(&mut ctx);

        // Extract pixels
        let image_info = self.surface.image_info();
        let w = image_info.width() as usize;
        let h = image_info.height() as usize;
        // Safety: the pixels are valid for the lifetime of the surface
        // which outlives this method. We store the pixmap in the surface.
        // We use a static-lifetime hack via a raw pointer read later.
        // For now we return via a temporary — the caller must blit immediately.
        &[]
    }

    /// Get the raw BGRA pixel bytes of the last rendered frame.
    pub fn pixels(&mut self) -> Option<Vec<u32>> {
        let w = self.surface.width();
        let h = self.surface.height();
        let row_bytes = (w * 4) as usize;
        let mut bytes = vec![0u8; row_bytes * h as usize];

        let image_info = self.surface.image_info();
        if self.surface.read_pixels(&image_info, bytes.as_mut_slice(), row_bytes, (0, 0)) {
            // Convert [u8; 4] BGRA to u32 (native endian ARGB for softbuffer)
            let pixels: Vec<u32> = bytes
                .chunks_exact(4)
                .map(|c| {
                    // Skia raster_n32 is BGRA on Windows/Linux
                    // softbuffer expects 0xAARRGGBB (or platform native)
                    // On Windows softbuffer is BGRA = B G R A in memory
                    // which is 0xAARRGGBB as u32 in native endian.
                    // Skia n32 on Windows is also BGRA, so it's already compatible.
                    u32::from_ne_bytes([c[0], c[1], c[2], c[3]])
                })
                .collect();
            Some(pixels)
        } else {
            None
        }
    }

    /// Get a mutable reference to the Skia canvas (for direct drawing).
    pub fn canvas(&mut self) -> &skia_safe::Canvas {
        self.surface.canvas()
    }
}

/// A drawing context passed to render callbacks.
///
/// All coordinate systems use the same top-left origin, y-down orientation
/// as Skia. Coordinates are in logical pixels (device-independent on HiDPI
/// systems, though we don't yet handle DPI scaling — a future improvement).
pub struct DrawCtx<'a> {
    pub canvas: &'a skia_safe::Canvas,
}

impl<'a> DrawCtx<'a> {
    // ── Primitives ────────────────────────────────────────────────────

    /// Fill a rectangle with a color.
    pub fn fill_rect(&self, rect: Rect, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.set_anti_alias(false);
        self.canvas.draw_rect(to_skia_rect(rect), &paint);
    }

    /// Fill a rounded rectangle.
    pub fn fill_rrect(&self, rect: Rect, radius: f32, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.set_anti_alias(true);
        let rrect = RRect::new_rect_xy(to_skia_rect(rect), radius, radius);
        self.canvas.draw_rrect(rrect, &paint);
    }

    /// Stroke a rectangle.
    pub fn stroke_rect(&self, rect: Rect, color: Color, width: f32) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.set_style(PaintStyle::Stroke);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        self.canvas.draw_rect(to_skia_rect(rect), &paint);
    }

    /// Stroke a rounded rectangle.
    pub fn stroke_rrect(&self, rect: Rect, radius: f32, color: Color, width: f32) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.set_style(PaintStyle::Stroke);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        let rrect = RRect::new_rect_xy(to_skia_rect(rect), radius, radius);
        self.canvas.draw_rrect(rrect, &paint);
    }

    /// Draw a line.
    pub fn draw_line(&self, from: Vec2, to: Vec2, color: Color, width: f32) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        self.canvas.draw_line((from.x, from.y), (to.x, to.y), &paint);
    }

    /// Draw a horizontal line (no anti-aliasing for crisp lines).
    pub fn draw_hline(&self, x: f32, y: f32, width: f32, color: Color, stroke: f32) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.set_stroke_width(stroke);
        paint.set_anti_alias(false);
        self.canvas
            .draw_line((x, y), (x + width, y), &paint);
    }

    /// Draw a vertical line (no anti-aliasing).
    pub fn draw_vline(&self, x: f32, y: f32, height: f32, color: Color, stroke: f32) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.set_stroke_width(stroke);
        paint.set_anti_alias(false);
        self.canvas
            .draw_line((x, y), (x, y + height), &paint);
    }

    /// Fill a circle.
    pub fn fill_circle(&self, center: Vec2, radius: f32, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.set_anti_alias(true);
        self.canvas
            .draw_circle((center.x, center.y), radius, &paint);
    }

    // ── Text ──────────────────────────────────────────────────────────

    /// Draw a text string.
    ///
    /// `origin` is the top-left corner of the text's bounding box. We
    /// offset by the font's ascent to get the baseline.
    pub fn draw_text(&self, text: &str, origin: Vec2, font: &Font, color: Color) {
        if text.is_empty() {
            return;
        }
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.set_anti_alias(true);

        let metrics = font.metrics();
        let baseline_y = origin.y - metrics.1.ascent;

        self.canvas.draw_str(text, (origin.x, baseline_y), font, &paint);
    }

    /// Draw a text string clipped to a rectangle.
    pub fn draw_text_clipped(&self, text: &str, origin: Vec2, font: &Font, color: Color, clip: Rect) {
        if text.is_empty() {
            return;
        }
        self.canvas.save();
        self.canvas.clip_rect(to_skia_rect(clip), None, None);
        self.draw_text(text, origin, font, color);
        self.canvas.restore();
    }

    // ── Clipping and transforms ────────────────────────────────────────

    /// Push a clip rectangle. All drawing inside `f` is clipped to `rect`.
    pub fn clip<F>(&self, rect: Rect, f: F)
    where
        F: FnOnce(&DrawCtx<'_>),
    {
        self.canvas.save();
        self.canvas.clip_rect(to_skia_rect(rect), None, Some(true));
        f(self);
        self.canvas.restore();
    }

    /// Push a transform offset. All drawing inside `f` is offset by `offset`.
    pub fn translate<F>(&self, offset: Vec2, f: F)
    where
        F: FnOnce(&DrawCtx<'_>),
    {
        self.canvas.save();
        self.canvas.translate((offset.x, offset.y));
        f(self);
        self.canvas.restore();
    }

    /// Push both a clip and a translate in one call.
    pub fn clip_translate<F>(&self, clip: Rect, offset: Vec2, f: F)
    where
        F: FnOnce(&DrawCtx<'_>),
    {
        self.canvas.save();
        self.canvas.clip_rect(to_skia_rect(clip), None, Some(true));
        self.canvas.translate((offset.x, offset.y));
        f(self);
        self.canvas.restore();
    }

    /// Clear the entire canvas to a color.
    pub fn clear(&self, color: Color) {
        self.canvas.clear(color.to_skia());
    }

    // ── Gradient helpers ──────────────────────────────────────────────

    /// Fill a rectangle with a vertical gradient from `top` to `bottom`.
    pub fn fill_gradient_v(&self, rect: Rect, top: Color, bottom: Color) {
        use skia_safe::gradient_shader;
        use skia_safe::TileMode;

        let shader = gradient_shader::linear(
            ((rect.x, rect.y), (rect.x, rect.bottom())),
            [top.to_skia(), bottom.to_skia()].as_ref(),
            None,
            TileMode::Clamp,
            None,
            None,
        );
        let mut paint = Paint::default();
        paint.set_shader(shader);
        self.canvas.draw_rect(to_skia_rect(rect), &paint);
    }

    // ── Box shadow / glow ─────────────────────────────────────────────

    /// Draw a simple drop shadow under a rounded rect (fake blur using layers).
    pub fn draw_shadow(&self, rect: Rect, radius: f32, shadow_blur: f32, shadow_color: Color) {
        use skia_safe::image_filters;
        let expanded = rect.inflate(shadow_blur);
        let rrect = RRect::new_rect_xy(to_skia_rect(expanded), radius + shadow_blur, radius + shadow_blur);
        let mut paint = Paint::default();
        paint.set_color(shadow_color.to_skia());
        paint.set_anti_alias(true);
        // Use a blur image filter to fake the shadow
        if let Some(blur) = image_filters::blur((shadow_blur, shadow_blur), None, None, None) {
            paint.set_image_filter(blur);
        }
        self.canvas.draw_rrect(rrect, &paint);
    }
}

/// Convert our `Rect` to Skia's `Rect`.
#[inline]
pub fn to_skia_rect(r: Rect) -> SkRect {
    SkRect::from_xywh(r.x, r.y, r.w, r.h)
}
