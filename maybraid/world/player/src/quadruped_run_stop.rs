//! Settle blend from quadruped trot contact back into still.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use crate::character::WALK_SPEED;

/// Keep in sync with `character_animations::animations::DEFAULT_QUADRUPED_RUN_STOP_SPEED`.
const SETTLE_SPEED: f32 = 3.5;
use crate::player::{Jumping, Player};

/// Progress from trot contact into quadruped still. `1.0` means settled.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct QuadrupedRunStopBlend {
	pub progress: f32,
}

impl Default for QuadrupedRunStopBlend {
	fn default() -> Self {
		Self { progress: 1.0 }
	}
}

impl QuadrupedRunStopBlend {
	pub fn settled(self) -> bool {
		self.progress >= 1.0
	}
}

/// Ease settle progress while still; reset when trotting or airborne.
pub(crate) fn advance_quadruped_run_stop_blend(
	time: Res<Time>,
	mut players: Query<(&LinearVelocity, Has<Jumping>, &mut QuadrupedRunStopBlend), With<Player>>,
) {
	let dt = time.delta_secs();
	for (velocity, jumping, mut blend) in &mut players {
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		if speed > WALK_SPEED || jumping {
			blend.progress = 0.0;
		} else if blend.progress < 1.0 {
			blend.progress = (blend.progress + dt * SETTLE_SPEED).min(1.0);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn blend_advances_when_still() {
		let mut blend = QuadrupedRunStopBlend { progress: 0.0 };
		blend.progress = (blend.progress + 0.1 * SETTLE_SPEED).min(1.0);
		assert!(blend.progress > 0.0 && blend.progress < 1.0);
	}

	#[test]
	fn blend_resets_when_trotting() {
		let mut blend = QuadrupedRunStopBlend { progress: 0.6 };
		let velocity = LinearVelocity(Vec3::new(3.0, 0.0, 0.0));
		if Vec3::new(velocity.x, 0.0, velocity.z).length() > WALK_SPEED {
			blend.progress = 0.0;
		}
		assert!(blend.progress.abs() < 1e-5);
	}

	#[test]
	fn default_is_settled() {
		assert!(QuadrupedRunStopBlend::default().settled());
	}
}
