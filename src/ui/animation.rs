//! Animation system for smooth, performance-conscious UI transitions.
//!
//! The system is built around a simple principle: each animation tracks a
//! value being interpolated from `from` to `to` over a duration with an
//! easing function. The manager updates all active animations once per frame
//! and reports whether any are still running (which tells the app to keep
//! requesting redraws).
//!
//! Animations are intentionally lightweight — no allocations per frame, no
//! complex timelines. For most UI effects (hover, button press, tab switch),
//! a single tween is enough.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::math::easing;
use crate::math::Lerp;

/// The state of an animation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimState {
    /// Not yet started.
    Idle,
    /// Currently running.
    Running { elapsed: f32, duration: f32 },
    /// Finished and holding the target value.
    Done,
}

/// A type alias for an easing function.
pub type EasingFn = fn(f32) -> f32;

/// A single animation interpolating a value over time.
///
/// Generic over the animated value type `T` (must implement `Lerp`).
/// For example, `Anim<f32>` animates a number, `Anim<Color>` animates a color.
#[derive(Clone)]
pub struct Anim<T: Lerp + Clone> {
    /// Starting value.
    from: T,
    /// Target value.
    to: T,
    /// Current state.
    state: AnimState,
    /// Easing function.
    easing: EasingFn,
}

impl<T: Lerp + Clone> Anim<T> {
    /// Create a new animation in the idle state at the given value.
    pub fn new(initial: T) -> Self {
        Self {
            from: initial.clone(),
            to: initial,
            state: AnimState::Idle,
            easing: easing::ease_in_out_cubic,
        }
    }

    /// Set the easing function.
    pub fn with_easing(mut self, easing: EasingFn) -> Self {
        self.easing = easing;
        self
    }

    /// Get the current animated value.
    pub fn value(&self) -> T {
        match self.state {
            AnimState::Idle => self.from.clone(),
            AnimState::Running { elapsed, duration } => {
                if duration <= 0.0 {
                    return self.to.clone();
                }
                let t = (elapsed / duration).clamp(0.0, 1.0);
                let eased = (self.easing)(t);
                self.from.lerp(&self.to, eased)
            }
            AnimState::Done => self.to.clone(),
        }
    }

    /// Get the raw progress (0.0 to 1.0), without easing.
    pub fn raw_progress(&self) -> f32 {
        match self.state {
            AnimState::Idle => 0.0,
            AnimState::Running { elapsed, duration } => {
                if duration <= 0.0 {
                    1.0
                } else {
                    (elapsed / duration).clamp(0.0, 1.0)
                }
            }
            AnimState::Done => 1.0,
        }
    }

    /// The eased progress (0.0 to 1.0).
    pub fn eased_progress(&self) -> f32 {
        (self.easing)(self.raw_progress())
    }

    /// Whether the animation is currently running.
    pub fn is_running(&self) -> bool {
        matches!(self.state, AnimState::Running { .. })
    }

    /// Whether the animation has finished.
    pub fn is_done(&self) -> bool {
        matches!(self.state, AnimState::Done)
    }

    /// Jump instantly to a target value (no animation).
    pub fn jump_to(&mut self, target: T) {
        self.from = target.clone();
        self.to = target;
        self.state = AnimState::Done;
    }

    /// Animate from the current value to a new target over `duration` seconds.
    pub fn animate_to(&mut self, target: T, duration: f32) {
        let current = self.value();
        self.from = current;
        self.to = target;
        self.state = AnimState::Running {
            elapsed: 0.0,
            duration,
        };
    }

    /// Update the animation by `dt` seconds. Returns true if still running.
    pub fn update(&mut self, dt: f32) -> bool {
        match &mut self.state {
            AnimState::Running { elapsed, duration } => {
                *elapsed += dt;
                if *elapsed >= *duration {
                    self.state = AnimState::Done;
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    /// Snap to the final value immediately.
    pub fn finish(&mut self) {
        if matches!(self.state, AnimState::Running { .. }) {
            self.state = AnimState::Done;
        }
    }

    /// Reset to the idle state at the current value.
    pub fn reset(&mut self) {
        let current = self.value();
        self.from = current.clone();
        self.to = current;
        self.state = AnimState::Idle;
    }
}

/// Manages named animations and provides time tracking.
///
/// Each frame, call `tick(dt)` to advance all running animations. If any
/// animation is still running, `needs_redraw()` will return true so the
/// app knows to keep rendering.
pub struct AnimationManager {
    /// Named float animations (hover progress, scroll offsets, etc.).
    floats: HashMap<String, Anim<f32>>,
    /// Timestamp of the last tick (for computing dt).
    last_time: Instant,
    /// Whether any animation was active during the last tick.
    any_active: bool,
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self {
            floats: HashMap::new(),
            last_time: Instant::now(),
            any_active: false,
        }
    }
}

impl AnimationManager {
    /// Create a new animation manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance all animations by the elapsed time since the last call.
    /// Returns whether any animation is still running.
    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let dt = now
            .saturating_duration_since(self.last_time)
            .as_secs_f32();
        self.last_time = now;

        // Cap dt to avoid huge jumps after stalls (e.g. debug breakpoints).
        let dt = dt.min(0.1);

        self.any_active = false;
        for anim in self.floats.values_mut() {
            if anim.update(dt) {
                self.any_active = true;
            }
        }
        self.any_active
    }

    /// Whether at least one animation is still running.
    #[inline]
    pub fn needs_redraw(&self) -> bool {
        self.any_active
    }

    /// Get or create a named float animation.
    pub fn float(&mut self, name: &str) -> &mut Anim<f32> {
        self.floats
            .entry(name.to_string())
            .or_insert_with(|| Anim::new(0.0))
    }

    /// Get a named float animation (read-only).
    pub fn get_float(&self, name: &str) -> Option<&Anim<f32>> {
        self.floats.get(name)
    }

    /// Set a float animation's target with duration.
    pub fn animate_float(&mut self, name: &str, target: f32, duration: f32) {
        let anim = self.float(name);
        anim.animate_to(target, duration);
    }

    /// Get the current value of a named float animation.
    pub fn float_value(&self, name: &str) -> Option<f32> {
        self.floats.get(name).map(|a| a.value())
    }

    /// Get a reference to the underlying map.
    pub fn floats(&self) -> &HashMap<String, Anim<f32>> {
        &self.floats
    }
}
