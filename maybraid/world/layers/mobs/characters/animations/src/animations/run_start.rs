//! One-shot still-to-run launch.
//!
//! Connects upright [`Idle`](super::Idle) at neutral rest to the first contact pose of
//! [`Run`](super::Run) at cycle phase `0`. Progress `0` is standing; `1` matches run@0.

use super::{smoothstep, Idle, Run};
use crate::Progress;

/// Default launch rate: full transition in ~0.22 s at speed `1.0`.
pub const DEFAULT_RUN_START_SPEED: f32 = 4.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunStart {
	pub idle: Idle,
	pub run: Run,
}

impl Default for RunStart {
	fn default() -> Self {
		Self { idle: Idle::default(), run: Run::default() }
	}
}

impl RunStart {
	/// Eased blend weight: 0 at upright rest, 1 at run phase zero.
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}
}
