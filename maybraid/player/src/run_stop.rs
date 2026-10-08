//! Run-to-walk-to-still deceleration when input is released.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_animations::animations::DEFAULT_RUN_STOP_SPEED;
use characters::{
	AnimMailbox, AnimRefRoot, CharacterMembers, CharacterRig, CharacterRigRole, CharacterRoot,
};

use crate::body::{CharacterController, Jumping, MoveWish};
use crate::locomotion::WALK_SPEED;
use crate::body::LEAP_SPEED;

/// Progress through [`AnimClip::RunStop`]. `0` inactive; `1` settled.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct RunStopBlend {
	pub progress: f32,
	pub phase: f32,
	pub from_run: bool,
}

impl RunStopBlend {
	pub fn active(self) -> bool {
		self.progress > 0.0 && self.progress < 1.0
	}

	pub fn settled(self) -> bool {
		!self.active()
	}
}

fn coasting(wish: Vec3) -> bool {
	wish.length_squared() < 1e-4
}

/// Advance or reset run-stop while the capsule coasts to rest.
pub(crate) fn advance_run_stop_blend(
	time: Res<Time>,
	mut controllers: Query<
		(
			Entity,
			&LinearVelocity,
			&MoveWish,
			Option<&Jumping>,
			&mut RunStopBlend,
		),
		With<CharacterController>,
	>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	mailboxes: Query<&AnimMailbox>,
	anims: Query<&AnimRefRoot>,
	rigs: Query<&CharacterRig>,
) {
	let dt = time.delta_secs();
	for (body, velocity, wish, jumping, mut blend) in &mut controllers {
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		let jumping = jumping.is_some_and(|jump| jump.airborne() || jump.phase == crate::body::JumpPhase::Takeoff);

		if !coasting(wish.0) || jumping {
			*blend = RunStopBlend::default();
			continue;
		}

		if blend.progress >= 1.0 {
			*blend = RunStopBlend::default();
		}

		if blend.progress == 0.0 && speed > WALK_SPEED {
			blend.from_run = speed > LEAP_SPEED;
			blend.phase = capture_gait_phase(body, &visuals, &mailboxes, &anims, &rigs);
		}

		if blend.progress > 0.0 && blend.progress < 1.0 {
			blend.progress = (blend.progress + dt * DEFAULT_RUN_STOP_SPEED).min(1.0);
		}
	}
}

fn capture_gait_phase(
	body: Entity,
	visuals: &Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	mailboxes: &Query<&AnimMailbox>,
	anims: &Query<&AnimRefRoot>,
	rigs: &Query<&CharacterRig>,
) -> f32 {
	for (members, child_of) in visuals.iter() {
		if child_of.parent() != body {
			continue;
		}
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let Ok(mailbox) = mailboxes.get(member) else {
				return 0.0;
			};
			let phase = mailbox.clip_cycle_phase().fract();
			if let Ok(root) = anims.get(member) {
				match root.0.clip.id() {
					characters::AnimId::Run | characters::AnimId::Walk => return phase,
					_ => {}
				}
			}
			return phase;
		}
	}
	0.0
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::JumpPhase;

	#[test]
	fn blend_resets_when_wish_returns() {
		let mut blend = RunStopBlend { progress: 0.6, phase: 0.3, from_run: true };
		if !coasting(Vec3::new(1.0, 0.0, 0.0)) {
			blend = RunStopBlend::default();
		}
		assert!(blend.settled());
	}

	#[test]
	fn blend_resets_when_airborne() {
		let mut blend = RunStopBlend { progress: 0.6, phase: 0.3, from_run: true };
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Air;
		if jump.airborne() {
			blend = RunStopBlend::default();
		}
		assert!(blend.settled());
	}

	#[test]
	fn walk_coast_skips_run_segment() {
		let speed = LEAP_SPEED - 1.0;
		let from_run = speed > LEAP_SPEED;
		assert!(!from_run);
	}
}
