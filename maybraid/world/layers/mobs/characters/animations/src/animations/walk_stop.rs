//! One-shot walk-to-still settle.
//!
//! Connects the first contact pose of [`Walk`](super::Walk) at cycle phase `0` back to neutral
//! [`Idle`](super::Idle) at rest. Progress `0` matches walk@0; `1` matches idle@0.

use super::{smoothstep, Idle, Walk};
use crate::Progress;

/// Default settle rate: full transition in ~0.28 s at speed `1.0`.
pub const DEFAULT_WALK_STOP_SPEED: f32 = 3.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkStop {
	pub walk: Walk,
	pub idle: Idle,
}

impl Default for WalkStop {
	fn default() -> Self {
		Self { walk: Walk::default(), idle: Idle::default() }
	}
}

impl WalkStop {
	/// Eased blend weight: 0 at walk phase zero, 1 at upright rest.
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}
}
