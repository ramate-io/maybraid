//! Advance trot launch progress on the body rig (animation-side component).

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use characters::{
	CharacterMembers, CharacterRig, CharacterRigRole, CharacterRoot, QuadrupedRunStartBlend,
};

use crate::body::{CharacterController, Jumping};
use crate::locomotion::WALK_SPEED;

/// Ease trot launch while moving in the jog band; reset when still, airborne, or galloping.
pub(crate) fn advance_quadruped_run_start_blend(
	time: Res<Time>,
	controllers: Query<(&LinearVelocity, Option<&Jumping>), With<CharacterController>>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<&CharacterRig>,
	mut blends: Query<&mut QuadrupedRunStartBlend>,
) {
	let dt = time.delta_secs();
	for (members, child_of) in &visuals {
		let Ok((velocity, jumping)) = controllers.get(child_of.parent()) else {
			continue;
		};
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let Ok(mut blend) = blends.get_mut(member) else {
				continue;
			};
			if speed <= WALK_SPEED
				|| jumping.is_some_and(|jump| jump.airborne())
				|| speed > crate::body::LEAP_SPEED
			{
				blend.progress = 0.0;
			} else if blend.progress < 1.0 {
				blend.progress = (blend.progress
					+ dt * character_animations::animations::DEFAULT_QUADRUPED_RUN_START_SPEED)
					.min(1.0);
			}
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
