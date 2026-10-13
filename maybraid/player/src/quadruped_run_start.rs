//! Trot launch blend from quadruped still into the run cycle.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_animations::animations::DEFAULT_QUADRUPED_RUN_START_SPEED;

use crate::body::{CharacterController, Jumping};
use crate::locomotion::WALK_SPEED;

/// Progress from quadruped idle into run phase zero. `1.0` means settled.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct QuadrupedRunStartBlend {
	pub progress: f32,
}

impl QuadrupedRunStartBlend {
	pub fn settled(self) -> bool {
		self.progress >= 1.0
	}
}

/// Ease trot launch while moving in the jog band; reset when still, airborne, or galloping.
pub(crate) fn advance_quadruped_run_start_blend(
	time: Res<Time>,
	mut controllers: Query<
		(&LinearVelocity, Option<&Jumping>, &mut QuadrupedRunStartBlend),
		With<CharacterController>,
	>,
) {
	let dt = time.delta_secs();
	for (velocity, jumping, mut blend) in &mut controllers {
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		if speed <= WALK_SPEED
			|| jumping.is_some_and(|jump| jump.airborne())
			|| speed > crate::body::LEAP_SPEED
		{
			blend.progress = 0.0;
		} else if blend.progress < 1.0 {
			blend.progress = (blend.progress + dt * DEFAULT_QUADRUPED_RUN_START_SPEED).min(1.0);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::{Jumping, LEAP_SPEED};

	#[test]
	fn blend_resets_when_still() {
		let mut blend = QuadrupedRunStartBlend { progress: 0.6 };
		let velocity = LinearVelocity(Vec3::ZERO);
		if Vec3::new(velocity.x, 0.0, velocity.z).length() <= WALK_SPEED {
			blend.progress = 0.0;
		}
		assert!(blend.progress.abs() < 1e-5);
	}

	#[test]
	fn blend_resets_when_airborne() {
		use crate::body::JumpPhase;

		let mut blend = QuadrupedRunStartBlend { progress: 0.6 };
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Air;
		if jump.airborne() {
			blend.progress = 0.0;
		}
		assert!(blend.progress.abs() < 1e-5);
	}

	#[test]
	fn blend_resets_when_galloping() {
		let mut blend = QuadrupedRunStartBlend { progress: 0.6 };
		let speed = LEAP_SPEED + 1.0;
		if speed > LEAP_SPEED {
			blend.progress = 0.0;
		}
		assert!(blend.progress.abs() < 1e-5);
	}
}
