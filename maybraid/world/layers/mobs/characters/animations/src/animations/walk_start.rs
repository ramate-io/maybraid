//! One-shot still-to-walk step-off.
//!
//! Connects upright [`Idle`](super::Idle) at neutral rest to the first contact pose of
//! [`Walk`](super::Walk) at cycle phase `0`. Progress `0` is standing; `1` matches walk@0.

use super::{smoothstep, Idle, Walk};
use crate::Progress;

/// Default step-off rate: full transition in ~0.28 s at speed `1.0`.
pub const DEFAULT_WALK_START_SPEED: f32 = 3.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkStart {
	pub idle: Idle,
	pub walk: Walk,
}

impl Default for WalkStart {
	fn default() -> Self {
		Self { idle: Idle::default(), walk: Walk::default() }
	}
}

impl WalkStart {
	/// Eased blend weight: 0 at upright rest, 1 at walk phase zero.
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}
}
