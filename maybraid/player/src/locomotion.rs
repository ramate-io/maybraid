//! Wish facing and walk/run/jump clips.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_animations::animations::smoothstep;
use characters::{
	AnimClip, AnimId, AnimProgress, AnimRef, AnimRefRoot, CharacterHeading, CharacterMembers,
	CharacterRig, CharacterRigRole, CharacterRoot, JumpParams, RigSkeletonKind,
};

use crate::body::{CharacterController, Jumping, MoveWish, JOG_SPEED, LEAP_SPEED};
use crate::identity::PlayerYawOwner;
use crate::stance::{CharacterStance, StanceKind};

pub(crate) const WALK_SPEED: f32 = 1.0;
/// Below this xz speed, squat stance keeps the frozen held pose.
pub(crate) const CROUCH_WALK_MIN_SPEED: f32 = 0.15;
/// Peak walk overlay in the squat+walk mix.
pub(crate) const CROUCH_WALK_MAX_WEIGHT: f32 = 0.40;

/// Leg-cycle influence while moving in squat stance (0 = frozen squat).
pub(crate) fn crouch_walk_weight(speed: f32) -> f32 {
	if speed <= CROUCH_WALK_MIN_SPEED {
		return 0.0;
	}
	let max_speed = JOG_SPEED * CharacterStance::settled(StanceKind::Squat).speed_scale();
	let span = (max_speed - CROUCH_WALK_MIN_SPEED).max(1e-3);
	smoothstep(((speed - CROUCH_WALK_MIN_SPEED) / span).clamp(0.0, 1.0)) * CROUCH_WALK_MAX_WEIGHT
}

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
		let kind = stance.map(|stance| stance.kind).unwrap_or(StanceKind::Stand);
		let blend = stance.map(|stance| stance.blend).unwrap_or(1.0);
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let clip = locomotion_clip(rig.skeleton, jumping, kind, blend, speed);
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
			} else if matches!(kind, StanceKind::Squat | StanceKind::Prone) {
				if clip.id() == AnimId::CrouchWalk {
					commands.entity(member).remove::<AnimProgress>();
				} else {
					commands.entity(member).insert(AnimProgress(blend));
				}
			} else {
				commands.entity(member).remove::<AnimProgress>();
			}
		}
	}
}

fn locomotion_clip(
	skeleton: RigSkeletonKind,
	jumping: Option<&Jumping>,
	stance: StanceKind,
	stance_blend: f32,
	speed: f32,
) -> AnimClip {
	match skeleton {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => {
			if jumping.is_some_and(|jump| jump.leaping) {
				AnimClip::leap()
			} else if jumping.is_some() {
				AnimClip::jump()
			} else {
				match stance {
					StanceKind::Prone => AnimClip::prone(),
					StanceKind::Squat => {
						if speed > 0.0 {
							// Keep CrouchWalk active (weight may be 0) so hovering near
							// CROUCH_WALK_MIN_SPEED does not restart mailbox crossfades.
							AnimClip::crouch_walk(stance_blend, crouch_walk_weight(speed))
						} else {
							AnimClip::squat()
						}
					}
					StanceKind::Stand => {
						if speed > LEAP_SPEED {
							AnimClip::run()
						} else if speed > WALK_SPEED {
							AnimClip::walk()
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
		assert!(!jump.leaping);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), StanceKind::Stand, 1.0, 0.0)
				.id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn running_leap_uses_leap_clip() {
		let jump = Jumping::start(MOVE_SPEED);
		assert!(jump.leaping);
		assert_eq!(
			locomotion_clip(
				RigSkeletonKind::Humanoid,
				Some(&jump),
				StanceKind::Stand,
				1.0,
				MOVE_SPEED,
			)
			.id(),
			AnimClip::leap().id()
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Quadruped, Some(&jump), StanceKind::Stand, 1.0, 0.0)
				.id(),
			AnimClip::leap().id()
		);
	}

	#[test]
	fn clip_follows_the_cap() {
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Stand, 1.0, JOG_SPEED)
				.id(),
			AnimId::Walk
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Stand, 1.0, MOVE_SPEED)
				.id(),
			AnimId::Run
		);
	}

	#[test]
	fn land_keeps_the_jump_clip() {
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Land;
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), StanceKind::Squat, 1.0, 0.0)
				.id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn stance_clips_win_over_still_walk() {
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Squat, 1.0, 0.0).id(),
			AnimId::Squat
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Prone, 1.0, 0.0).id(),
			AnimId::Prone
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Stand, 1.0, 0.0).id(),
			AnimId::Still
		);
		let jump = Jumping::start(0.0);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), StanceKind::Squat, 1.0, 0.0)
				.id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn squat_moving_uses_crouch_walk() {
		let clip = locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Squat, 1.0, 2.0);
		assert_eq!(clip.id(), AnimId::CrouchWalk);
		let weight = match clip {
			AnimClip::CrouchWalk(params) => params.walk_weight,
			_ => panic!("expected crouch walk"),
		};
		assert!(weight > 0.0 && weight < CROUCH_WALK_MAX_WEIGHT);
	}

	#[test]
	fn squat_still_stays_frozen() {
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Squat, 1.0, 0.0).id(),
			AnimId::Squat
		);
	}

	#[test]
	fn squat_below_threshold_keeps_crouch_walk_id() {
		let clip = locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Squat, 1.0, 0.10);
		assert_eq!(clip.id(), AnimId::CrouchWalk);
		let weight = match clip {
			AnimClip::CrouchWalk(params) => params.walk_weight,
			_ => panic!("expected crouch walk"),
		};
		assert_eq!(weight, 0.0);
	}

	#[test]
	fn squat_threshold_band_does_not_flicker_clip_id() {
		let below = locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Squat, 1.0, 0.14);
		let above = locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Squat, 1.0, 0.16);
		assert_eq!(below.id(), AnimId::CrouchWalk);
		assert_eq!(above.id(), AnimId::CrouchWalk);
	}

	#[test]
	fn jump_from_crouch_walk_uses_jump_clip() {
		let jump = Jumping::start(2.0);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), StanceKind::Squat, 1.0, 2.0)
				.id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn stand_up_from_crouch_walk_uses_standing_clip() {
		let clip = locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Stand, 1.0, 2.0);
		assert_eq!(clip.id(), AnimId::Walk);
	}

	#[test]
	fn crouch_walk_weight_endpoints() {
		assert_eq!(crouch_walk_weight(0.0), 0.0);
		assert_eq!(crouch_walk_weight(CROUCH_WALK_MIN_SPEED), 0.0);
		let max_speed = JOG_SPEED * 0.5;
		assert!((crouch_walk_weight(max_speed) - CROUCH_WALK_MAX_WEIGHT).abs() < 1e-5);
	}
}
