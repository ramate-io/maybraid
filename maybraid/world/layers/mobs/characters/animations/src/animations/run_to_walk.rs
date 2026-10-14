//! One-shot run-to-walk deceleration.
//!
//! Connects [`Run`](super::Run) to [`Walk`](super::Walk) at a shared gait phase so
//! the legs stay in stride while bounce and arm swing ease toward walk energy.
//! Progress `0` is full run; `1` is full walk at the same phase.
//! Sampling is stateless: progress alone sets the blend weight.

use super::{smoothstep, Run, Walk};
use crate::Progress;

/// Default shift rate: full transition in ~0.25 s at speed `1.0`.
pub const DEFAULT_RUN_TO_WALK_SPEED: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunToWalk {
	pub run: Run,
	pub walk: Walk,
	/// Shared gait cycle phase for both endpoints (`0..1`).
	pub gait_phase: f32,
}

impl Default for RunToWalk {
	fn default() -> Self {
		Self { run: Run::default(), walk: Walk::default(), gait_phase: 0.0 }
	}
}

impl RunToWalk {
	/// Eased blend weight: 0 at run, 1 at walk (same `gait_phase`).
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	pub fn gait_phase(&self) -> f32 {
		self.gait_phase.rem_euclid(1.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn weight_endpoints() -> anyhow::Result<()> {
		let shift = RunToWalk::default();
		assert!(shift.weight(0.0).abs() < 1e-5);
		assert!((shift.weight(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn weight_eases_in_the_middle() -> anyhow::Result<()> {
		let shift = RunToWalk::default();
		let mid = shift.weight(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn weight_clamps_past_one() -> anyhow::Result<()> {
		let shift = RunToWalk::default();
		assert!((shift.weight(1.5) - 1.0).abs() < 1e-5);
		Ok(())
	}
}
