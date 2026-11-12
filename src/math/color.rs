//! Color type with conversion to Skia's Color and common color operations.

use super::Lerp;

/// An 8-bit RGBA color.
///
/// Stored as separate channels for easy manipulation in themes and
/// animation interpolation. Converts to Skia's `Color` when painting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Convert to Skia's `Color` (ARGB packed u32).
    #[inline]
    pub fn to_skia(self) -> skia_safe::Color {
        skia_safe::Color::from_argb(self.a, self.r, self.g, self.b)
    }

    /// Parse a hex color string like `#1a2b3c` or `#1a2b3cff`.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a))
            }
            _ => None,
        }
    }

    /// Convert to HSL and adjust lightness, then convert back.
    /// Useful for generating hover/active states from a base color.
    pub fn adjust_lightness(self, delta: f32) -> Self {
        let (h, s, l) = self.to_hsl();
        let new_l = (l + delta).clamp(0.0, 1.0);
        Self::from_hsl(h, s, new_l, self.a)
    }

    /// Convert to HSL (hue 0-360, saturation and lightness 0.0-1.0).
    pub fn to_hsl(self) -> (f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let l = (max + min) / 2.0;

        if delta == 0.0 {
            return (0.0, 0.0, l);
        }

        let s = if l < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };

        let h = if max == r {
            ((g - b) / delta) % 6.0
        } else if max == g {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };
        let h = h * 60.0;
        let h = if h < 0.0 { h + 360.0 } else { h };

        (h, s, l)
    }

    /// Create from HSL values (hue 0-360, saturation and lightness 0.0-1.0).
    pub fn from_hsl(h: f32, s: f32, l: f32, a: u8) -> Self {
        if s == 0.0 {
            let v = (l * 255.0) as u8;
            return Self::rgba(v, v, v, a);
        }

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r1, g1, b1) = match h {
            h if h < 60.0 => (c, x, 0.0),
            h if h < 120.0 => (x, c, 0.0),
            h if h < 180.0 => (0.0, c, x),
            h if h < 240.0 => (0.0, x, c),
            h if h < 300.0 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self::rgba(
            ((r1 + m) * 255.0) as u8,
            ((g1 + m) * 255.0) as u8,
            ((b1 + m) * 255.0) as u8,
            a,
        )
    }

    /// Premultiply alpha into the RGB channels (for premultiplied buffers).
    #[inline]
    pub fn premultiply(self) -> Self {
        let a = self.a as f32 / 255.0;
        Self::rgba(
            (self.r as f32 * a) as u8,
            (self.g as f32 * a) as u8,
            (self.b as f32 * a) as u8,
            self.a,
        )
    }

    /// Blend this color over another (source-over compositing).
    #[inline]
    pub fn blend_over(self, background: Self) -> Self {
        let sa = self.a as f32 / 255.0;
        let da = background.a as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a == 0.0 {
            return Self::TRANSPARENT;
        }
        let blend = |s: u8, d: u8| -> u8 {
            let s = s as f32;
            let d = d as f32;
            ((s * sa + d * da * (1.0 - sa)) / out_a) as u8
        };
        Self::rgba(
            blend(self.r, background.r),
            blend(self.g, background.g),
            blend(self.b, background.b),
            (out_a * 255.0) as u8,
        )
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl Lerp for Color {
    #[inline]
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let lerp_u8 = |a: u8, b: u8| -> u8 {
            (a as f32 + (b as f32 - a as f32) * t) as u8
        };
        Self::rgba(
            lerp_u8(self.r, other.r),
            lerp_u8(self.g, other.g),
            lerp_u8(self.b, other.b),
            lerp_u8(self.a, other.a),
        )
    }
}
