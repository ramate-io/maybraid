//! Step-off blend from still into the walk cycle.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_animations::animations::DEFAULT_WALK_START_SPEED;

use crate::body::{CharacterController, Jumping};
use crate::locomotion::WALK_SPEED;

/// Progress from upright still into walk phase zero. `1.0` means settled.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct WalkStartBlend {
	pub progress: f32,
}

impl WalkStartBlend {
	pub fn settled(self) -> bool {
		self.progress >= 1.0
	}
}

/// Ease step-off progress while moving; reset when still or airborne.
pub(crate) fn advance_walk_start_blend(
	time: Res<Time>,
	mut controllers: Query<
		(&LinearVelocity, Option<&Jumping>, &mut WalkStartBlend),
		With<CharacterController>,
	>,
) {
	let dt = time.delta_secs();
	for (velocity, jumping, mut blend) in &mut controllers {
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		if speed <= WALK_SPEED || jumping.is_some_and(|jump| jump.airborne()) {
			blend.progress = 0.0;
		} else if blend.progress < 1.0 {
			blend.progress = (blend.progress + dt * DEFAULT_WALK_START_SPEED).min(1.0);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::Jumping;

	#[test]
	fn blend_resets_when_still() {
		let mut blend = WalkStartBlend { progress: 0.6 };
		let velocity = LinearVelocity(Vec3::ZERO);
		if Vec3::new(velocity.x, 0.0, velocity.z).length() <= WALK_SPEED {
			blend.progress = 0.0;
		}
		assert!(blend.progress.abs() < 1e-5);
	}

	#[test]
	fn blend_resets_when_airborne() {
		use crate::body::JumpPhase;

		let mut blend = WalkStartBlend { progress: 0.6 };
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Air;
		if jump.airborne() {
			blend.progress = 0.0;
		}
		assert!(blend.progress.abs() < 1e-5);
	}
}
