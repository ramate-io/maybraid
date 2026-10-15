//! Settle blend from walk contact back into still.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_animations::animations::DEFAULT_WALK_STOP_SPEED;

use crate::body::{CharacterController, Jumping};
use crate::locomotion::WALK_SPEED;

/// Progress from walk contact into upright still. `1.0` means settled.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WalkStopBlend {
	pub progress: f32,
}

impl Default for WalkStopBlend {
	fn default() -> Self {
		Self { progress: 1.0 }
	}
}

impl WalkStopBlend {
	pub fn settled(self) -> bool {
		self.progress >= 1.0
	}
}

/// Ease settle progress while still; reset when moving or airborne.
pub(crate) fn advance_walk_stop_blend(
	time: Res<Time>,
	mut controllers: Query<
		(&LinearVelocity, Option<&Jumping>, &mut WalkStopBlend),
		With<CharacterController>,
	>,
) {
	let dt = time.delta_secs();
	for (velocity, jumping, mut blend) in &mut controllers {
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		if speed > WALK_SPEED || jumping.is_some_and(|jump| jump.airborne()) {
			blend.progress = 0.0;
		} else if blend.progress < 1.0 {
			blend.progress = (blend.progress + dt * DEFAULT_WALK_STOP_SPEED).min(1.0);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::{JumpPhase, Jumping};

	#[test]
	fn blend_advances_when_still() {
		let mut blend = WalkStopBlend { progress: 0.0 };
		blend.progress = (blend.progress + 0.1 * DEFAULT_WALK_STOP_SPEED).min(1.0);
		assert!(blend.progress > 0.0 && blend.progress < 1.0);
	}

	#[test]
	fn blend_resets_when_moving() {
		let mut blend = WalkStopBlend { progress: 0.6 };
		let velocity = LinearVelocity(Vec3::new(3.0, 0.0, 0.0));
		if Vec3::new(velocity.x, 0.0, velocity.z).length() > WALK_SPEED {
			blend.progress = 0.0;
		}
		assert!(blend.progress.abs() < 1e-5);
	}

	#[test]
	fn blend_resets_when_airborne() {
		let mut blend = WalkStopBlend { progress: 0.6 };
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Air;
		if jump.airborne() {
			blend.progress = 0.0;
		}
		assert!(blend.progress.abs() < 1e-5);
	}

	#[test]
	fn default_is_settled() {
		assert!(WalkStopBlend::default().settled());
	}
}
