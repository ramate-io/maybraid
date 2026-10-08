//! Wish facing and walk/run/jump clips.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use characters::{
	AnimClip, AnimProgress, AnimRef, AnimRefRoot, CharacterHeading, CharacterMembers, CharacterRig,
	CharacterRigRole, CharacterRoot, JumpParams, RigSkeletonKind,
};

use crate::body::{CharacterController, Jumping, MoveWish, LEAP_SPEED};
use crate::identity::PlayerYawOwner;
use crate::stance::{CharacterStance, StanceKind};
use crate::walk_stop::WalkStopBlend;

pub(crate) const WALK_SPEED: f32 = 1.0;

pub(crate) fn face_wish_yaw(
	time: Res<Time>,
	wishes: Query<&MoveWish, With<CharacterController>>,
	mut visuals: Query<
		(&mut Transform, &mut CharacterHeading, Option<&PlayerYawOwner>, &ChildOf),
		With<CharacterRoot>,
	>,
) {
	for (mut visual, mut heading, owner, child_of) in &mut visuals {
		if owner.copied().unwrap_or_default() != PlayerYawOwner::Wish {
			continue;
		}
		let Ok(wish) = wishes.get(child_of.parent()) else {
			continue;
		};
		heading.turn_toward(&mut visual, wish.0, time.delta_secs());
	}
}

pub fn drive_player_locomotion(
	mut commands: Commands,
	controllers: Query<
		(
			&LinearVelocity,
			&MoveWish,
			Option<&Jumping>,
			Option<&CharacterStance>,
			Option<&WalkStopBlend>,
		),
		With<CharacterController>,
	>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	for (members, child_of) in &visuals {
		let Ok((velocity, _wish, jumping, stance, walk_stop)) = controllers.get(child_of.parent())
		else {
			continue;
		};
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		let stance = stance.copied().unwrap_or_else(|| CharacterStance::settled(StanceKind::Stand));
		let walk_stop = walk_stop.copied().unwrap_or_default();
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let clip = locomotion_clip(rig.skeleton, jumping, &stance, &walk_stop, speed);
			let desired = AnimRef::new(clip);
			let needs = match anims.get(member) {
				Ok(root) => root.0 != desired,
				Err(_) => true,
			};
			if needs {
				commands.entity(member).insert(AnimRefRoot(desired));
			}
			if let Some(jump) = jumping {
				let phase = jump.leap_progress(velocity.y);
				let progress = if matches!(clip, AnimClip::Leap(_)) {
					phase
				} else {
					JumpParams::default().elapsed_from_phase(phase)
				};
				commands.entity(member).insert(AnimProgress(progress));
			} else if !stance.squat_settled() {
				commands.entity(member).insert(AnimProgress(stance.blend));
			} else if matches!(stance.kind, StanceKind::Squat | StanceKind::Prone) {
				commands.entity(member).insert(AnimProgress(stance.blend));
			} else if matches!(clip, AnimClip::WalkStop) {
				commands.entity(member).insert(AnimProgress(walk_stop.progress));
			} else {
				commands.entity(member).remove::<AnimProgress>();
			}
		}
	}
}

fn locomotion_clip(
	skeleton: RigSkeletonKind,
	jumping: Option<&Jumping>,
	stance: &CharacterStance,
	walk_stop: &WalkStopBlend,
	speed: f32,
) -> AnimClip {
	match skeleton {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => {
			if jumping.is_some_and(|jump| jump.leaping) {
				AnimClip::leap()
			} else if jumping.is_some() {
				AnimClip::jump()
			} else if !stance.squat_settled() {
				AnimClip::squat_descent()
			} else {
				match stance.kind {
					StanceKind::Prone => AnimClip::prone(),
					StanceKind::Squat => AnimClip::squat(),
					StanceKind::Stand => {
						if speed > LEAP_SPEED {
							AnimClip::run()
						} else if speed > WALK_SPEED {
							AnimClip::walk()
						} else if !walk_stop.settled() {
							AnimClip::walk_stop()
						} else {
							AnimClip::still()
						}
					}
				}
			}
		}
		RigSkeletonKind::Quadruped => {
			if jumping.is_some() {
				AnimClip::leap()
			} else if speed > LEAP_SPEED {
				AnimClip::gallop()
			} else if speed > WALK_SPEED {
				AnimClip::quadruped_run()
			} else {
				AnimClip::still()
			}
		}
		RigSkeletonKind::Forelimbed => {
			if speed > LEAP_SPEED {
				AnimClip::dorsoventral_undulation()
			} else if speed > WALK_SPEED {
				AnimClip::lateral_undulation()
			} else {
				AnimClip::still()
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::{JumpPhase, JOG_SPEED, MOVE_SPEED};
	use characters::AnimId;

	#[test]
	fn standing_hop_uses_jump_clip() {
		let jump = Jumping::start(0.0);
		let stance = CharacterStance::settled(StanceKind::Stand);
		let walk_stop = WalkStopBlend::default();
		assert!(!jump.leaping);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), &stance, &walk_stop, 0.0).id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn running_leap_uses_leap_clip() {
		let jump = Jumping::start(MOVE_SPEED);
		let stance = CharacterStance::settled(StanceKind::Stand);
		let walk_stop = WalkStopBlend::default();
		assert!(jump.leaping);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				Some(&jump),
				&stance,
				&walk_stop,
				MOVE_SPEED,
			)
			.id(),
			AnimClip::leap().id()
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Quadruped, Some(&jump), &stance, &walk_stop, 0.0).id(),
			AnimClip::leap().id()
		);
	}

	#[test]
	fn clip_follows_the_cap() {
		let stance = CharacterStance::settled(StanceKind::Stand);
		let walk_stop = WalkStopBlend::default();
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, &walk_stop, JOG_SPEED).id(),
			AnimId::Walk
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, &walk_stop, MOVE_SPEED).id(),
			AnimId::Run
		);
	}

	#[test]
	fn land_keeps_the_jump_clip() {
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Land;
		let stance = CharacterStance::settled(StanceKind::Squat);
		let walk_stop = WalkStopBlend::default();
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), &stance, &walk_stop, 0.0).id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn stance_clips_win_over_still_walk() {
		let walk_stop = WalkStopBlend::default();
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&CharacterStance::settled(StanceKind::Squat),
				&walk_stop,
				0.0,
			)
			.id(),
			AnimId::Squat
		);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&CharacterStance::settled(StanceKind::Prone),
				&walk_stop,
				0.0,
			)
			.id(),
			AnimId::Prone
		);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&CharacterStance::settled(StanceKind::Stand),
				&walk_stop,
				0.0,
			)
			.id(),
			AnimId::Still
		);
		let jump = Jumping::start(0.0);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				Some(&jump),
				&CharacterStance::settled(StanceKind::Squat),
				&walk_stop,
				0.0,
			)
			.id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn entering_squat_uses_descent_clip() {
		let stance = CharacterStance { kind: StanceKind::Squat, blend: 0.0 };
		let walk_stop = WalkStopBlend::default();
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, &walk_stop, 0.0).id(),
			AnimId::SquatDescent
		);
	}

	#[test]
	fn leaving_squat_uses_descent_until_blend_reaches_zero() {
		let stance = CharacterStance { kind: StanceKind::Stand, blend: 0.5 };
		let walk_stop = WalkStopBlend::default();
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, &walk_stop, 0.0).id(),
			AnimId::SquatDescent
		);
	}

	#[test]
	fn stopping_uses_walk_stop_until_settled() {
		let stance = CharacterStance::settled(StanceKind::Stand);
		let stopping = WalkStopBlend { progress: 0.0 };
		let settled = WalkStopBlend::default();
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, &stopping, 0.0).id(),
			AnimId::WalkStop
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, &settled, 0.0).id(),
			AnimId::Still
		);
	}

	#[test]
	fn jump_interrupts_walk_stop() {
		let jump = Jumping::start(0.0);
		let stance = CharacterStance::settled(StanceKind::Stand);
		let walk_stop = WalkStopBlend { progress: 0.4 };
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), &stance, &walk_stop, 0.0,).id(),
			AnimClip::jump().id()
		);
	}
}
