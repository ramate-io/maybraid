//! Crouched locomotion with a held squat depth and an independent gait overlay.

use super::Walk;

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn blended_controls_clamp_and_keep_authored_walk_tuning() -> anyhow::Result<()> {
		let crouch = CrouchedWalk::blended(1.5, -0.2);
		assert_eq!(crouch.depth, 1.0);
		assert_eq!(crouch.leg_cycle, 0.0);
		assert_eq!(crouch.walk, Walk { stride: 0.20, bounce: 0.40, rotation: 0.30 });
		Ok(())
	}
}
