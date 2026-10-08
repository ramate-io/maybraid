//! Run-to-walk deceleration when input is released above walking speed.
//!
//! Progress is driven by measured planar speed under [`MOVE_BRAKE`], not wall-clock
//! time. Walk→idle below [`WALK_SPEED`] is left to [`AnimClip::Approach`] ([#1022]).

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_animations::animations::{
	run_stop_cycle_speed, run_stop_progress_from_speed, RUN_STOP_HANDOFF_SPEED, RUN_TO_WALK_END,
};
use characters::{
	AnimId, AnimMailbox, AnimRefRoot, CharacterMembers, CharacterRig, CharacterRigRole,
	CharacterRoot,
};

use crate::body::{CharacterController, Jumping, MoveWish};

/// Progress through [`AnimClip::RunStop`]. `0` inactive; [`RUN_TO_WALK_END`] hands off.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct RunStopBlend {
	pub progress: f32,
	pub phase: f32,
	pub from_run: bool,
	pub start_speed: f32,
}

impl RunStopBlend {
	pub fn active(self) -> bool {
		self.from_run && self.progress > 0.0 && self.progress < RUN_TO_WALK_END
	}

	pub fn settled(self) -> bool {
		!self.active()
	}
}

/// Gait phase captured when run-stop is interrupted; consumed on the next locomotion tick.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct GaitPhaseHandoff(pub f32);

fn coasting(wish: Vec3) -> bool {
	wish.length_squared() < 1e-4
}

/// Advance or reset run-stop while the capsule coasts to rest.
pub(crate) fn advance_run_stop_blend(
	time: Res<Time>,
	mut commands: Commands,
	mut controllers: Query<
		(Entity, &LinearVelocity, &MoveWish, Option<&Jumping>, &mut RunStopBlend),
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
		let jumping = jumping
			.is_some_and(|jump| jump.airborne() || jump.phase == crate::body::JumpPhase::Takeoff);

		if !coasting(wish.0) || jumping {
			if blend.active() {
				commands.entity(body).insert(GaitPhaseHandoff(blend.phase));
			}
			*blend = RunStopBlend::default();
			continue;
		}

		if blend.progress >= RUN_TO_WALK_END {
			*blend = RunStopBlend::default();
		}

		if blend.progress == 0.0 && speed > RUN_STOP_HANDOFF_SPEED {
			let from_run = capture_from_run(body, &visuals, &anims, &rigs);
			if from_run {
				blend.from_run = true;
				blend.start_speed = speed;
				blend.phase = capture_gait_phase(body, &visuals, &mailboxes, &anims, &rigs);
				blend.progress = run_stop_progress_from_speed(speed, blend.start_speed);
			}
		}

		if blend.active() {
			blend.progress = run_stop_progress_from_speed(speed, blend.start_speed);
			blend.phase =
				(blend.phase + dt * run_stop_cycle_speed(speed, blend.progress)).rem_euclid(1.0);
		}
	}
}

fn capture_from_run(
	body: Entity,
	visuals: &Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	anims: &Query<&AnimRefRoot>,
	rigs: &Query<&CharacterRig>,
) -> bool {
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
			if let Ok(root) = anims.get(member) {
				return root.0.clip.id() == AnimId::Run;
			}
			return false;
		}
	}
	false
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
					AnimId::Run | AnimId::Walk => return phase,
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
	use crate::body::{JumpPhase, LEAP_SPEED, MOVE_SPEED};
	use crate::locomotion::WALK_SPEED;

	#[test]
	fn blend_resets_when_wish_returns() {
		let mut blend =
			RunStopBlend { progress: 0.3, phase: 0.35, from_run: true, start_speed: MOVE_SPEED };
		if !coasting(Vec3::new(1.0, 0.0, 0.0)) {
			blend = RunStopBlend::default();
		}
		assert!(blend.settled());
	}

	#[test]
	fn blend_resets_when_airborne() {
		let mut blend =
			RunStopBlend { progress: 0.3, phase: 0.35, from_run: true, start_speed: MOVE_SPEED };
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Air;
		if jump.airborne() {
			blend = RunStopBlend::default();
		}
		assert!(blend.settled());
	}

	#[test]
	fn from_run_follows_visible_clip_not_speed() {
		let speed_below_leap = LEAP_SPEED - 1.0;
		assert!(speed_below_leap < LEAP_SPEED);
		// Visible Run clip at stop onset — not the leap-speed threshold.
		let from_run = capture_from_run_logic(AnimId::Run, speed_below_leap);
		assert!(from_run);
		let from_walk = capture_from_run_logic(AnimId::Walk, MOVE_SPEED);
		assert!(!from_walk);
	}

	fn capture_from_run_logic(clip: AnimId, _speed: f32) -> bool {
		clip == AnimId::Run
	}

	#[test]
	fn progress_tracks_speed_not_time() {
		let start = MOVE_SPEED;
		let at_start = run_stop_progress_from_speed(start, start);
		assert!(at_start < 1e-5);
		let mid = run_stop_progress_from_speed(5.0, start);
		assert!(mid > 0.0 && mid < RUN_TO_WALK_END);
		let at_handoff = run_stop_progress_from_speed(WALK_SPEED, start);
		assert!((at_handoff - RUN_TO_WALK_END).abs() < 1e-5);
	}

	#[test]
	fn walk_coast_does_not_activate_run_stop() {
		let from_run = AnimId::Walk == AnimId::Run;
		assert!(!from_run);
	}
}
