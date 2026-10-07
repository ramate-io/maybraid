//! Wish facing and walk/run/jump clips.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use characters::{
	approach_walk_weight, AnimClip, AnimId, AnimProgress, AnimRef, AnimRefRoot, CharacterHeading,
	CharacterMembers, CharacterRig, CharacterRigRole, CharacterRoot, JumpParams, RigSkeletonKind,
};

use crate::body::{CharacterController, Jumping, MoveWish, LEAP_SPEED};
use crate::identity::PlayerYawOwner;
use crate::stance::{CharacterStance, StanceKind};

pub(crate) const WALK_SPEED: f32 = 1.0;
/// Enter [`AnimClip::Approach`] above this planar speed (m/s).
pub(crate) const APPROACH_ENTER_SPEED: f32 = 0.1;
/// Leave approach for still below this planar speed (m/s).
pub(crate) const APPROACH_EXIT_SPEED: f32 = 0.05;
/// Leave walk for approach below this planar speed (m/s).
pub(crate) const WALK_EXIT_SPEED: f32 = 0.92;

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
		(&LinearVelocity, &MoveWish, Option<&Jumping>, Option<&CharacterStance>),
		With<CharacterController>,
	>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	for (members, child_of) in &visuals {
		let Ok((velocity, _wish, jumping, stance)) = controllers.get(child_of.parent()) else {
			continue;
		};
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		let stance = stance.copied().unwrap_or_else(|| CharacterStance::settled(StanceKind::Stand));
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let current = anims.get(member).ok().map(|root| root.0.clip.id());
			let clip = locomotion_clip(rig.skeleton, jumping, &stance, speed, current);
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
	speed: f32,
	current: Option<AnimId>,
) -> AnimClip {
	let was_approach = current.is_some_and(|id| id == AnimId::Approach);
	let was_walking = current.is_some_and(|id| matches!(id, AnimId::Walk));
	let in_approach =
		if was_approach { speed >= APPROACH_EXIT_SPEED } else { speed > APPROACH_ENTER_SPEED };
	let walking = if was_walking { speed > WALK_EXIT_SPEED } else { speed > WALK_SPEED };
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
						} else if walking {
							AnimClip::walk()
						} else if in_approach {
							AnimClip::approach(approach_walk_weight(speed))
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
		assert!(!jump.leaping);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), &stance, 0.0, None).id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn running_leap_uses_leap_clip() {
		let jump = Jumping::start(MOVE_SPEED);
		let stance = CharacterStance::settled(StanceKind::Stand);
		assert!(jump.leaping);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), &stance, MOVE_SPEED, None).id(),
			AnimClip::leap().id()
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Quadruped, Some(&jump), &stance, 0.0, None).id(),
			AnimClip::leap().id()
		);
	}

	#[test]
	fn approach_blends_idle_into_walk_by_speed() {
		let stance = CharacterStance::settled(StanceKind::Stand);
		let creep = locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, 0.5, None);
		assert_eq!(creep.id(), AnimId::Approach);
		let walk = locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, JOG_SPEED, None);
		assert_eq!(walk.id(), AnimId::Walk);
		let sprint = locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, MOVE_SPEED, None);
		assert_eq!(sprint.id(), AnimId::Run);
		let creep_weight = match creep {
			AnimClip::Approach(params) => params.walk_weight,
			_ => panic!("expected approach"),
		};
		assert!(creep_weight > 0.0 && creep_weight < 1.0);
		assert_eq!(approach_walk_weight(0.0), 0.0);
		assert!((approach_walk_weight(WALK_SPEED) - 1.0).abs() < 1e-5);
	}

	#[test]
	fn physics_jitter_stays_still_without_flapping() {
		let stance = CharacterStance::settled(StanceKind::Stand);
		let jitter = 1e-5_f32;
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, jitter, Some(AnimId::Still))
				.id(),
			AnimId::Still
		);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&stance,
				jitter,
				Some(AnimId::Approach)
			)
			.id(),
			AnimId::Still
		);
	}

	#[test]
	fn approach_entry_and_exit_use_hysteresis() {
		let stance = CharacterStance::settled(StanceKind::Stand);
		let mid = (APPROACH_ENTER_SPEED + APPROACH_EXIT_SPEED) * 0.5;
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, mid, Some(AnimId::Still))
				.id(),
			AnimId::Still
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, mid, Some(AnimId::Approach))
				.id(),
			AnimId::Approach
		);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&stance,
				APPROACH_ENTER_SPEED + 1e-4,
				None,
			)
			.id(),
			AnimId::Approach
		);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&stance,
				APPROACH_EXIT_SPEED - 1e-4,
				Some(AnimId::Approach),
			)
			.id(),
			AnimId::Still
		);
	}

	#[test]
	fn walk_boundary_uses_hysteresis() {
		let stance = CharacterStance::settled(StanceKind::Stand);
		let mid = (WALK_SPEED + WALK_EXIT_SPEED) * 0.5;
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, mid, Some(AnimId::Approach))
				.id(),
			AnimId::Approach
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, mid, Some(AnimId::Walk)).id(),
			AnimId::Walk
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, WALK_SPEED + 1e-4, None,)
				.id(),
			AnimId::Walk
		);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&stance,
				WALK_EXIT_SPEED - 1e-4,
				Some(AnimId::Walk),
			)
			.id(),
			AnimId::Approach
		);
	}

	#[test]
	fn drive_player_locomotion_reads_velocity_only() {
		let src = include_str!("locomotion.rs");
		let body = src
			.split("pub fn drive_player_locomotion")
			.nth(1)
			.and_then(|tail| tail.split("fn locomotion_clip").next())
			.expect("drive_player_locomotion");
		assert_eq!(body.matches("LinearVelocity").count(), 1, "planar speed from velocity");
		assert!(body.contains("_wish"), "MoveWish is ignored for clip selection");
		assert!(!body.contains("LinearVelocity("), "must not construct velocity");
		assert!(!body.contains("ExternalForce"), "no physics writes");
		assert!(!body.contains("ExternalImpulse"), "no physics writes");
		assert!(!body.contains("Position"), "no displacement writes");
	}

	#[test]
	fn land_keeps_the_jump_clip() {
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Land;
		let stance = CharacterStance::settled(StanceKind::Squat);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), &stance, 0.0, None).id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn stance_clips_win_over_still_walk() {
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&CharacterStance::settled(StanceKind::Squat),
				0.0,
				None,
			)
			.id(),
			AnimId::Squat
		);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&CharacterStance::settled(StanceKind::Prone),
				0.0,
				None,
			)
			.id(),
			AnimId::Prone
		);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				None,
				&CharacterStance::settled(StanceKind::Stand),
				0.0,
				None,
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
				0.0,
				None,
			)
			.id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn entering_squat_uses_descent_clip() {
		let stance = CharacterStance { kind: StanceKind::Squat, blend: 0.0 };
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, 0.0, None).id(),
			AnimId::SquatDescent
		);
	}

	#[test]
	fn leaving_squat_uses_descent_until_blend_reaches_zero() {
		let stance = CharacterStance { kind: StanceKind::Stand, blend: 0.5 };
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, 0.0, None).id(),
			AnimId::SquatDescent
		);
	}
}
