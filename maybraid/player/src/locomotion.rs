//! Wish facing and walk/run/jump clips.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_animations::animations::smoothstep;
use characters::{
	AnimClip, AnimProgress, AnimRef, AnimRefRoot, CharacterHeading, CharacterMembers, CharacterRig,
	CharacterRigRole, CharacterRoot, JumpParams, RigSkeletonKind,
};

use crate::body::{CharacterController, Jumping, MoveWish, LEAP_SPEED};
use crate::identity::PlayerYawOwner;
use crate::stance::{CharacterStance, StanceKind};

pub(crate) const WALK_SPEED: f32 = 1.0;

/// 0 = walk, 1 = run. Smoothsteps between [`WALK_SPEED`] and [`LEAP_SPEED`].
pub(crate) fn gait_run_weight(speed: f32) -> f32 {
	if speed <= WALK_SPEED {
		return 0.0;
	}
	let span = (LEAP_SPEED - WALK_SPEED).max(1e-3);
	smoothstep(((speed - WALK_SPEED) / span).clamp(0.0, 1.0))
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
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let clip = locomotion_clip(rig.skeleton, jumping, kind, speed);
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
				commands
					.entity(member)
					.insert(AnimProgress(stance.map(|s| s.blend).unwrap_or(1.0)));
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
					StanceKind::Squat => AnimClip::squat(),
					StanceKind::Stand => {
						if speed > WALK_SPEED {
							AnimClip::gait(gait_run_weight(speed))
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
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), StanceKind::Stand, 0.0).id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn running_leap_uses_leap_clip() {
		let jump = Jumping::start(MOVE_SPEED);
		assert!(jump.leaping);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), StanceKind::Stand, MOVE_SPEED)
				.id(),
			AnimClip::leap().id()
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Quadruped, Some(&jump), StanceKind::Stand, 0.0).id(),
			AnimClip::leap().id()
		);
	}

	#[test]
	fn gait_blends_walk_into_run_by_speed() {
		let jog = locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Stand, JOG_SPEED);
		assert_eq!(jog.id(), AnimId::Gait);
		let sprint =
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Stand, MOVE_SPEED);
		assert_eq!(sprint.id(), AnimId::Gait);
		let jog_weight = match jog {
			AnimClip::Gait(params) => params.run_weight,
			_ => panic!("expected gait"),
		};
		let sprint_weight = match sprint {
			AnimClip::Gait(params) => params.run_weight,
			_ => panic!("expected gait"),
		};
		assert!(jog_weight > 0.0 && jog_weight < 1.0);
		assert!(sprint_weight > jog_weight);
		assert!((sprint_weight - 1.0).abs() < 1e-3);
	}

	#[test]
	fn gait_weight_endpoints() {
		assert_eq!(gait_run_weight(0.0), 0.0);
		assert_eq!(gait_run_weight(WALK_SPEED), 0.0);
		assert!((gait_run_weight(LEAP_SPEED) - 1.0).abs() < 1e-5);
		assert!((gait_run_weight(MOVE_SPEED) - 1.0).abs() < 1e-5);
	}

	#[test]
	fn land_keeps_the_jump_clip() {
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Land;
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), StanceKind::Squat, 0.0).id(),
			AnimClip::jump().id()
		);
	}

	#[test]
	fn stance_clips_win_over_still_walk() {
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Squat, 0.0).id(),
			AnimId::Squat
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Prone, 0.0).id(),
			AnimId::Prone
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, StanceKind::Stand, 0.0).id(),
			AnimId::Still
		);
		let jump = Jumping::start(0.0);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, Some(&jump), StanceKind::Squat, 0.0).id(),
			AnimClip::jump().id()
		);
	}
}
