//! Rectangles, sizes, and insets — the geometric building blocks for layout.

use super::Vec2;

/// A 2D size: width and height.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Size {
    pub w: f32,
    pub h: f32,
}

impl Size {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const INFINITY: Self = Self::new(f32::INFINITY, f32::INFINITY);

    #[inline]
    pub const fn new(w: f32, h: f32) -> Self {
        Self { w, h }
    }

    #[inline]
    pub fn area(self) -> f32 {
        self.w * self.h
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    #[inline]
    pub fn shrink(self, insets: Insets) -> Self {
        Self::new(
            (self.w - insets.horizontal()).max(0.0),
            (self.h - insets.vertical()).max(0.0),
        )
    }
}

/// A rectangle with top-left corner at (x, y) and dimensions (w, h).
///
/// The coordinate system is y-down, matching Skia and most windowing
/// toolkits.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    #[inline]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    #[inline]
    pub const fn from_size(size: Size) -> Self {
        Self::new(0.0, 0.0, size.w, size.h)
    }

    #[inline]
    pub const fn from_pos_size(pos: Vec2, size: Size) -> Self {
        Self::new(pos.x, pos.y, size.w, size.h)
    }

    #[inline]
    pub fn left(self) -> f32 {
        self.x
    }

    #[inline]
    pub fn top(self) -> f32 {
        self.y
    }

    #[inline]
    pub fn right(self) -> f32 {
        self.x + self.w
    }

    #[inline]
    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    #[inline]
    pub fn center(self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    #[inline]
    pub fn size(self) -> Size {
        Size::new(self.w, self.h)
    }

    #[inline]
    pub fn origin(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    #[inline]
    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x < self.right()
            && point.y >= self.y
            && point.y < self.bottom()
    }

    #[inline]
    pub fn inflate(self, amount: f32) -> Self {
        Self::new(
            self.x - amount,
            self.y - amount,
            self.w + amount * 2.0,
            self.h + amount * 2.0,
        )
    }

    #[inline]
    pub fn deflate(self, insets: Insets) -> Self {
        Self::new(
            self.x + insets.left,
            self.y + insets.top,
            (self.w - insets.horizontal()).max(0.0),
            (self.h - insets.vertical()).max(0.0),
        )
    }

    #[inline]
    pub fn translate(self, offset: Vec2) -> Self {
        Self::new(self.x + offset.x, self.y + offset.y, self.w, self.h)
    }

    #[inline]
    pub fn with_size(self, size: Size) -> Self {
        Self::new(self.x, self.y, size.w, size.h)
    }

    #[inline]
    pub fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}

/// Insets / padding for each edge of a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    pub const ZERO: Self = Self::all(0.0);

    #[inline]
    pub const fn all(v: f32) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    #[inline]
    pub const fn symmetrical(horizontal: f32, vertical: f32) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }

    #[inline]
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    #[inline]
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    #[inline]
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}
