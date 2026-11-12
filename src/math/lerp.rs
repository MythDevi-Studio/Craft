//! Linear interpolation trait and implementations for primitive types.

/// Trait for types that can be interpolated between two values.
///
/// Used by the animation system to smoothly transition between states.
pub trait Lerp: Sized {
    /// Interpolate between `self` and `other` by factor `t` (0.0 = self, 1.0 = other).
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    #[inline]
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for f64 {
    #[inline]
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t as f64
    }
}

impl Lerp for u8 {
    #[inline]
    fn lerp(&self, other: &Self, t: f32) -> Self {
        (*self as f32 + (*other as f32 - *self as f32) * t) as u8
    }
}

impl Lerp for usize {
    #[inline]
    fn lerp(&self, other: &Self, t: f32) -> Self {
        (*self as f32 + (*other as f32 - *self as f32) * t) as usize
    }
}

impl<T: Lerp> Lerp for Option<T> {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        match (self, other) {
            (Some(a), Some(b)) => Some(a.lerp(b, t)),
            (Some(a), None) if t < 0.5 => Some(a.clone()),
            (None, Some(b)) if t >= 0.5 => Some(b.clone()),
            _ => None,
        }
    }
}
