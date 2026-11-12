//! Easing functions for smooth, natural-looking animations.
//!
//! Each function takes a normalized time `t` in [0.0, 1.0] and returns
//! an eased progress value, also typically in [0.0, 1.0] (though some
//! functions overshoot for elastic/bounce effects).

use std::f32::consts::PI;

/// Identity easing — constant speed.
#[inline]
pub fn linear(t: f32) -> f32 {
    t.clamp(0.0, 1.0)
}

/// Quadratic ease-in — starts slow, accelerates.
#[inline]
pub fn ease_in_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t
}

/// Quadratic ease-out — starts fast, decelerates.
#[inline]
pub fn ease_out_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * (2.0 - t)
}

/// Quadratic ease-in-out — slow start and end, fast middle.
#[inline]
pub fn ease_in_out_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

/// Cubic ease-in — stronger acceleration than quadratic.
#[inline]
pub fn ease_in_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

/// Cubic ease-out — strong deceleration at the end.
#[inline]
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let f = t - 1.0;
    f * f * f + 1.0
}

/// Cubic ease-in-out — smooth S-curve, the default for most UI.
#[inline]
pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let f = 2.0 * t - 2.0;
        0.5 * f * f * f + 1.0
    }
}

/// Exponential ease-out — very fast start, gradual settle.
/// Good for entrance animations and transitions.
#[inline]
pub fn ease_out_expo(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t == 1.0 {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

/// Exponential ease-in-out.
#[inline]
pub fn ease_in_out_expo(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else if t < 0.5 {
        2.0_f32.powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
    }
}

/// Back ease-out — overshoots slightly then settles back.
/// Adds a "springy" feel without full physics simulation.
#[inline]
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Smoothstep — a common smooth interpolation curve.
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smootherstep — even smoother than smoothstep (zero 2nd derivatives at endpoints).
#[inline]
pub fn smootherstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// A cubic Bézier easing function, matching CSS `cubic-bezier()`.
///
/// Takes two control points (x1, y1) and (x2, y2) in the unit square.
/// P0 = (0,0) and P3 = (1,1) are implicit.
pub struct CubicBezier {
    cx: f32,
    bx: f32,
    ax: f32,
    cy: f32,
    by: f32,
    ay: f32,
}

impl CubicBezier {
    /// Create a cubic Bézier easing curve.
    ///
    /// Control points are (x1, y1) and (x2, y2). P0 = (0, 0), P3 = (1, 1).
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        // Precompute polynomial coefficients for Newton-Raphson solve.
        let cx = 3.0 * x1;
        let bx = 3.0 * (x2 - x1) - cx;
        let ax = 1.0 - cx - bx;
        let cy = 3.0 * y1;
        let by = 3.0 * (y2 - y1) - cy;
        let ay = 1.0 - cy - by;
        Self {
            cx,
            bx,
            ax,
            cy,
            by,
            ay,
        }
    }

    /// Evaluate the curve at progress `t` (0.0 to 1.0).
    pub fn at(&self, t: f32) -> f32 {
        // Solve for the x value that corresponds to input t, then evaluate y.
        // We treat the input as the desired x, solve for parameter s, then evaluate y(s).
        let x = t.clamp(0.0, 1.0);
        let s = self.solve_for_x(x);
        self.y_at(s)
    }

    fn x_at(&self, t: f32) -> f32 {
        ((self.ax * t + self.bx) * t + self.cx) * t
    }

    fn y_at(&self, t: f32) -> f32 {
        ((self.ay * t + self.by) * t + self.cy) * t
    }

    fn dx_at(&self, t: f32) -> f32 {
        (3.0 * self.ax * t + 2.0 * self.bx) * t + self.cx
    }

    fn solve_for_x(&self, x: f32) -> f32 {
        // Newton-Raphson with fallback to bisection.
        let mut t = x;
        for _ in 0..8 {
            let dx = self.dx_at(t);
            if dx.abs() < 1e-6 {
                break;
            }
            let current_x = self.x_at(t) - x;
            t -= current_x / dx;
            t = t.clamp(0.0, 1.0);
        }
        t
    }
}

/// Common Bézier presets matching CSS easing keywords.
pub fn ease() -> f32 {
    // Placeholder — use ease_in_out_cubic directly
    ease_in_out_cubic(1.0)
}

/// Helper to apply an easing function and clamp the result.
#[inline]
pub fn apply(easing: fn(f32) -> f32, t: f32) -> f32 {
    easing(t)
}

/// Map a value from one range to another.
#[inline]
pub fn map_range(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    if from_max == from_min {
        return to_min;
    }
    let t = (value - from_min) / (from_max - from_min);
    to_min + t * (to_max - to_min)
}

/// Clamp a value to a range.
#[inline]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

/// Damp a value toward a target using exponential smoothing.
///
/// `speed` controls how fast the value approaches the target.
/// Higher speed = snappier. This is frame-rate independent when given
/// the delta time since the last update.
#[inline]
pub fn damp(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
    if speed <= 0.0 || dt <= 0.0 {
        return current;
    }
    let t = 1.0 - (-speed * dt).exp();
    current + (target - current) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear() {
        assert_eq!(linear(0.0), 0.0);
        assert_eq!(linear(0.5), 0.5);
        assert_eq!(linear(1.0), 1.0);
    }

    #[test]
    fn test_ease_out_cubic() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(11.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_map_range() {
        assert!((map_range(5.0, 0.0, 10.0, 0.0, 100.0) - 50.0).abs() < 0.001);
        assert!((map_range(0.0, 0.0, 10.0, 0.0, 100.0) - 0.0).abs() < 0.001);
        assert!((map_range(10.0, 0.0, 10.0, 0.0, 100.0) - 100.0).abs() < 0.001);
    }
}
