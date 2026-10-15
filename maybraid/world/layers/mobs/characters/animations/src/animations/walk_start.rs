//! One-shot still-to-walk step-off.
//!
//! Connects upright [`Idle`](super::Idle) at neutral rest to the first contact pose of
//! [`Walk`](super::Walk) at cycle phase `0`. Progress `0` is standing; `1` matches walk@0.
//! Sampling is stateless: progress alone sets the blend weight.

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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn weight_endpoints() -> anyhow::Result<()> {
		let start = WalkStart::default();
		assert!(start.weight(0.0).abs() < 1e-5);
		assert!((start.weight(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn weight_eases_in_the_middle() -> anyhow::Result<()> {
		let start = WalkStart::default();
		let mid = start.weight(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn weight_clamps_past_one() -> anyhow::Result<()> {
		let start = WalkStart::default();
		assert!((start.weight(1.5) - 1.0).abs() < 1e-5);
		Ok(())
	}
}
