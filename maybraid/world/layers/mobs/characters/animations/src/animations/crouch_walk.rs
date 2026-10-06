//! Crouched locomotion: held squat depth with an explicit leg-cycle overlay.
//!
//! [`CrouchedWalk`] is not a full-pose [`Mix`] with standing [`Walk`]. `leg_cycle`
//! scales only the gait oscillation on top of the squat-depth knobs.

use super::Walk;

#[derive(Debug, Clone, PartialEq)]
pub struct CrouchedWalk {
	pub walk: Walk,
	/// Held squat depth (0 = stand, 1 = bottom).
	pub depth: f32,
	/// Leg-cycle influence (0 = frozen squat, 1 = full stride overlay).
	pub leg_cycle: f32,
}

impl CrouchedWalk {
	pub fn blended(depth: f32, leg_cycle: f32) -> Self {
		Self {
			walk: Walk { stride: 0.20, bounce: 0.40, rotation: 0.30 },
			depth: depth.clamp(0.0, 1.0),
			leg_cycle: leg_cycle.clamp(0.0, 1.0),
		}
	}
}
