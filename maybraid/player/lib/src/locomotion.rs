//! Wish facing and walk/run/jump clips.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use characters::{
	AnimClip, AnimId, AnimMailbox, AnimProgress, AnimRef, AnimRefRoot, CharacterHeading,
	CharacterMembers, CharacterRig, CharacterRigRole, CharacterRoot, JumpParams, RigSkeletonKind,
};

use crate::body::{
	CharacterController, Grounded, Jumping, MoveWish, JOG_SPEED, LEAP_SPEED, MOVE_SPEED,
};
use crate::identity::PlayerYawOwner;
use crate::stance::{CharacterStance, StanceKind};

pub(crate) const WALK_SPEED: f32 = 1.0;
const STAND_WALK_ENTER: f32 = 1.12;
const STAND_MOVE_ENTER: f32 = 0.08;
const STAND_MOVE_EXIT: f32 = 0.04;

/// Rest/moving hysteresis. The mailbox owns time; speed only changes pose weights.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StandLocomotionGait {
	band: StandLocomotionBand,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StandLocomotionBand {
	#[default]
	Still,
	Moving,
}

impl StandLocomotionBand {
	fn advance(self, speed: f32) -> Self {
		let threshold = match self {
			Self::Still => STAND_MOVE_ENTER,
			Self::Moving => STAND_MOVE_EXIT,
		};
		if speed > threshold {
			Self::Moving
		} else {
			Self::Still
		}
	}

	fn clip(self, speed: f32) -> AnimClip {
		match self {
			Self::Still => AnimClip::still(),
			Self::Moving => AnimClip::grounded_gait(
				(speed / STAND_WALK_ENTER).clamp(0.0, 1.0),
				((speed - JOG_SPEED) / (MOVE_SPEED - JOG_SPEED)).clamp(0.0, 1.0),
			),
		}
	}
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
		if !heading.would_turn_toward(&visual, wish.0) {
			continue;
		}
		heading.turn_toward(&mut visual, wish.0, time.delta_secs());
	}
}

pub fn drive_player_locomotion(
	mut commands: Commands,
	controllers: Query<
		(&LinearVelocity, &MoveWish, Option<&Jumping>, Option<&CharacterStance>, Has<Grounded>),
		With<CharacterController>,
	>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<&CharacterRig>,
	anims: Query<(&AnimRefRoot, Option<&AnimMailbox>)>,
	gait: Query<&StandLocomotionGait>,
) {
	for (members, child_of) in &visuals {
		let Ok((velocity, _wish, jumping, stance, grounded)) = controllers.get(child_of.parent())
		else {
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
			let stand_band = if rig.skeleton == RigSkeletonKind::Humanoid
				&& grounded && jumping.is_none()
				&& stance.squat_settled()
				&& matches!(stance.kind, StanceKind::Stand)
			{
				let band = gait.get(member).map(|g| g.band).unwrap_or_default().advance(speed);
				commands.entity(member).try_insert(StandLocomotionGait { band });
				Some(band)
			} else {
				commands.entity(member).try_remove::<StandLocomotionGait>();
				None
			};
			let current = anims.get(member).ok();
			let clip = locomotion_clip(rig.skeleton, jumping, &stance, speed, stand_band);
			let clip = if jumping.is_none() {
				let current_clip = current.map(|(root, _)| root.0.clip);
				let progress = current
					.and_then(|(_, mailbox)| mailbox)
					.map_or(0.0, AnimMailbox::clip_progress);
				match rig.skeleton {
					RigSkeletonKind::Humanoid => {
						stance_clip(clip, current_clip, progress, stance.kind)
					}
					RigSkeletonKind::Quadruped
						if grounded
							&& stance.squat_settled()
							&& stance.kind == StanceKind::Stand =>
					{
						quadruped_transition(clip, current_clip, progress)
					}
					_ => clip,
				}
			} else {
				clip
			};
			let desired = AnimRef::new(clip);
			let needs = current.is_none_or(|(root, _)| root.0 != desired);
			if needs {
				commands.entity(member).try_insert(AnimRefRoot(desired));
			}
			if let Some(jump) = jumping {
				let phase = jump.leap_progress(velocity.y);
				let progress = if matches!(clip, AnimClip::Leap(_)) {
					phase
				} else {
					JumpParams::default().elapsed_from_phase(phase)
				};
				commands.entity(member).insert(AnimProgress(progress));
			} else {
				let progress = match clip.id() {
					AnimId::SquatDescent => Some(stance.blend),
					AnimId::SquatAscent => Some(1.0 - stance.blend),
					AnimId::Squat | AnimId::Prone => Some(1.0),
					_ => None,
				};
				if let Some(progress) = progress {
					commands.entity(member).try_insert(AnimProgress(progress));
				} else {
					// Crouch walking and authored transitions use the mailbox clock.
					commands.entity(member).try_remove::<AnimProgress>();
				}
			}
		}
	}
}

fn locomotion_clip(
	skeleton: RigSkeletonKind,
	jumping: Option<&Jumping>,
	stance: &CharacterStance,
	speed: f32,
	stand_gait: Option<StandLocomotionBand>,
) -> AnimClip {
	match skeleton {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => {
			if jumping.is_some_and(|jump| jump.leaping) {
				AnimClip::leap()
			} else if jumping.is_some() {
				AnimClip::jump()
			} else if !stance.squat_settled() {
				if stance.kind == StanceKind::Stand {
					AnimClip::squat_ascent()
				} else {
					AnimClip::squat_descent()
				}
			} else {
				match stance.kind {
					StanceKind::Prone => AnimClip::prone(),
					StanceKind::Squat if speed > STAND_MOVE_ENTER => AnimClip::crouch_walk(),
					StanceKind::Squat => AnimClip::squat(),
					StanceKind::Stand => stand_gait
						.map(|band| band.clip(speed))
						.unwrap_or_else(|| stand_locomotion_clip_direct(speed)),
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

fn stand_locomotion_clip_direct(speed: f32) -> AnimClip {
	StandLocomotionBand::Still.advance(speed).clip(speed)
}

/// Stance identity owns entry/exit, while mailbox progress owns their duration.
fn stance_clip(
	desired: AnimClip,
	current: Option<AnimClip>,
	progress: f32,
	stance: StanceKind,
) -> AnimClip {
	let Some(current) = current else {
		return desired;
	};
	match (stance, current.id()) {
		(StanceKind::Prone, AnimId::Prone) => desired,
		(StanceKind::Prone, AnimId::ProneDescent) if progress >= 1.0 => AnimClip::prone(),
		(StanceKind::Prone, _) => AnimClip::prone_descent(),
		(_, AnimId::Prone | AnimId::ProneDescent) => AnimClip::prone_ascent(),
		(_, AnimId::ProneAscent) if progress < 1.0 => current,
		_ => desired,
	}
}

fn quadruped_transition(desired: AnimClip, current: Option<AnimClip>, progress: f32) -> AnimClip {
	let Some(current) = current else {
		return desired;
	};
	match desired.id() {
		AnimId::Still if current.id().needs_quadruped_run_stop(progress) => {
			AnimClip::quadruped_run_stop()
		}
		AnimId::QuadrupedRun => match current.id() {
			AnimId::Still => AnimClip::quadruped_run_start(),
			AnimId::QuadrupedRunStart if progress < 1.0 => current,
			_ => desired,
		},
		AnimId::Gallop => match current.id() {
			AnimId::QuadrupedRun => AnimClip::quadruped_run_to_gallop(progress),
			AnimId::QuadrupedRunToGallop if progress < 1.0 => current,
			_ => desired,
		},
		_ => desired,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::JumpPhase;
	use bevy::ecs::system::RunSystemOnce;
	use characters::AnimId;

	#[test]
	fn production_quadruped_trot_stops_once_and_allows_movement_or_jump() -> anyhow::Result<()> {
		use characters::{tick_anim_mailbox, MemberOf};
		use std::time::Duration;

		let mut world = World::new();
		let mut time = Time::<()>::default();
		time.advance_by(Duration::from_secs_f32(0.05));
		world.insert_resource(time);
		let controller = world
			.spawn((
				CharacterController,
				LinearVelocity(Vec3::X * 3.0),
				MoveWish(Vec3::ZERO),
				Grounded,
			))
			.id();
		let visual = world.spawn((CharacterRoot, ChildOf(controller))).id();
		let body = world
			.spawn((
				MemberOf(visual),
				CharacterRig { role: CharacterRigRole::Body, skeleton: RigSkeletonKind::Quadruped },
				AnimRefRoot(AnimRef::new(AnimClip::quadruped_run())),
				AnimMailbox::new(Transform::IDENTITY),
				AnimProgress(0.37),
			))
			.id();
		let mut schedule = Schedule::default();
		schedule.add_systems((drive_player_locomotion, tick_anim_mailbox).chain());
		let selected = |world: &World| -> anyhow::Result<AnimId> {
			Ok(world
				.get::<AnimRefRoot>(body)
				.ok_or_else(|| anyhow::anyhow!("missing body clip"))?
				.0
				.clip
				.id())
		};
		// Begin at an arbitrary trot clock phase, then let the production mailbox clock run.
		world.run_system_once(tick_anim_mailbox).map_err(|e| anyhow::anyhow!("{e:?}"))?;
		world.entity_mut(body).remove::<AnimProgress>();
		world.entity_mut(controller).insert(LinearVelocity(Vec3::X * MOVE_SPEED));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::QuadrupedRunToGallop);
		for _ in 0..8 {
			schedule.run(&mut world);
		}
		assert_eq!(selected(&world)?, AnimId::Gallop, "acceleration hands off once");
		world.entity_mut(controller).insert(LinearVelocity(Vec3::X * 3.0));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::QuadrupedRun);
		world.entity_mut(controller).insert(LinearVelocity(Vec3::ZERO));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::QuadrupedRunStop);
		world.entity_mut(controller).insert(LinearVelocity(Vec3::X * 3.0));
		schedule.run(&mut world);
		assert_eq!(
			selected(&world)?,
			AnimId::QuadrupedRun,
			"movement interrupts the unfinished stop"
		);
		world.entity_mut(controller).insert(LinearVelocity(Vec3::ZERO));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::QuadrupedRunStop);
		for _ in 0..8 {
			schedule.run(&mut world);
		}
		assert_eq!(selected(&world)?, AnimId::Still);
		for _ in 0..4 {
			schedule.run(&mut world);
		}
		assert_eq!(selected(&world)?, AnimId::Still, "idle does not repeatedly restart stopping");
		world.entity_mut(controller).insert(LinearVelocity(Vec3::X * 3.0));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::QuadrupedRunStart);
		for _ in 0..8 {
			schedule.run(&mut world);
		}
		assert_eq!(selected(&world)?, AnimId::QuadrupedRun);
		world.entity_mut(controller).insert(LinearVelocity(Vec3::ZERO));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::QuadrupedRunStop);
		world.entity_mut(controller).insert(Jumping::start(3.0));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::Leap);
		world.entity_mut(controller).remove::<Jumping>();
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::Still, "landing must not synthesize a trot stop");
		for stance in [
			CharacterStance::settled(StanceKind::Prone),
			CharacterStance { kind: StanceKind::Squat, blend: 0.4 },
		] {
			world.entity_mut(controller).insert((
				LinearVelocity(Vec3::X * 3.0),
				CharacterStance::settled(StanceKind::Stand),
			));
			for _ in 0..8 {
				schedule.run(&mut world);
			}
			world.entity_mut(controller).insert(LinearVelocity(Vec3::ZERO));
			schedule.run(&mut world);
			assert_eq!(selected(&world)?, AnimId::QuadrupedRunStop);
			world.entity_mut(controller).insert(stance);
			for _ in 0..4 {
				schedule.run(&mut world);
				assert_eq!(
					selected(&world)?,
					AnimId::Still,
					"stance progress must not pin the stop clock"
				);
			}
		}
		Ok(())
	}

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
	fn continuous_gait_reaches_walk_and_run_at_their_speed_caps() -> anyhow::Result<()> {
		for (speed, expected_run) in [(JOG_SPEED, 0.0), (MOVE_SPEED, 1.0)] {
			let AnimClip::GroundedGait(gait) = stand_locomotion_clip_direct(speed) else {
				anyhow::bail!("moving humanoid must use the continuous gait");
			};
			assert_eq!(gait.walk_weight, 1.0);
			assert_eq!(gait.run_weight, expected_run);
		}
		Ok(())
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
	fn rest_hysteresis_does_not_chatter_and_gait_identity_survives_speed_changes() {
		let mut band = StandLocomotionBand::Still;
		for speed in [0.03, 0.07, 0.05] {
			band = band.advance(speed);
			assert_eq!(band.clip(speed).id(), AnimId::Still);
		}
		for speed in [0.1, 0.06, 0.09, 1.0, 1.2, 7.49, 7.51, MOVE_SPEED] {
			band = band.advance(speed);
			assert_eq!(band.clip(speed).id(), AnimId::GroundedGait);
		}
		assert_eq!(band.advance(0.02).clip(0.02).id(), AnimId::Still);
	}

	#[test]
	fn production_gait_and_stance_transitions_share_the_mailbox_clock() -> anyhow::Result<()> {
		use characters::{tick_anim_mailbox, MemberOf};
		use std::time::Duration;
		let mut world = World::new();
		let mut time = Time::<()>::default();
		time.advance_by(Duration::from_secs_f32(0.05));
		world.insert_resource(time);
		let controller = world
			.spawn((CharacterController, LinearVelocity(Vec3::X), MoveWish(Vec3::ZERO), Grounded))
			.id();
		let visual = world.spawn((CharacterRoot, ChildOf(controller))).id();
		let body = world
			.spawn((
				MemberOf(visual),
				CharacterRig::default(),
				AnimRefRoot(AnimRef::new(AnimClip::still())),
				AnimMailbox::new(Transform::IDENTITY),
			))
			.id();
		let mut schedule = Schedule::default();
		schedule.add_systems((drive_player_locomotion, tick_anim_mailbox).chain());
		let selected = |world: &World| -> anyhow::Result<AnimId> {
			Ok(world
				.get::<AnimRefRoot>(body)
				.ok_or_else(|| anyhow::anyhow!("missing clip"))?
				.0
				.clip
				.id())
		};
		let progress = |world: &World| -> anyhow::Result<f32> {
			Ok(world
				.get::<AnimMailbox>(body)
				.ok_or_else(|| anyhow::anyhow!("missing mailbox"))?
				.clip_progress())
		};
		schedule.run(&mut world);
		let mut previous = progress(&world)?;
		for speed in [0.9, 1.1, 6.0, 7.6, 10.5, 1.0] {
			world.entity_mut(controller).insert(LinearVelocity(Vec3::X * speed));
			schedule.run(&mut world);
			assert_eq!(selected(&world)?, AnimId::GroundedGait);
			assert!(progress(&world)? > previous, "retuning weights must not restart the clock");
			previous = progress(&world)?;
			assert!(world.get::<AnimProgress>(body).is_none());
		}
		world.entity_mut(controller).insert(CharacterStance::settled(StanceKind::Squat));
		world.entity_mut(body).insert(AnimProgress(1.0));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::CrouchWalk);
		assert!(
			world.get::<AnimProgress>(body).is_none(),
			"held squat progress must not freeze walking"
		);
		assert!(world.get::<StandLocomotionGait>(body).is_none());
		let first_crouch = progress(&world)?;
		schedule.run(&mut world);
		assert!(progress(&world)? > first_crouch);
		world.entity_mut(controller).insert(CharacterStance::settled(StanceKind::Prone));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::ProneDescent);
		assert!(world.get::<AnimProgress>(body).is_none());
		for _ in 0..10 {
			schedule.run(&mut world);
		}
		assert_eq!(selected(&world)?, AnimId::Prone);
		world.entity_mut(controller).insert(CharacterStance::settled(StanceKind::Stand));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::ProneAscent);
		assert!(world.get::<AnimProgress>(body).is_none());
		for _ in 0..10 {
			schedule.run(&mut world);
		}
		assert_eq!(selected(&world)?, AnimId::GroundedGait);
		world.entity_mut(controller).insert(CharacterStance::settled(StanceKind::Prone));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::ProneDescent);
		world.entity_mut(controller).insert(Jumping::start(3.0));
		schedule.run(&mut world);
		assert_eq!(selected(&world)?, AnimId::Jump);
		assert!(world.get::<StandLocomotionGait>(body).is_none());
		Ok(())
	}

	#[test]
	fn leaving_squat_uses_ascent_until_blend_reaches_zero() {
		let stance = CharacterStance { kind: StanceKind::Stand, blend: 0.5 };
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, 0.0, None).id(),
			AnimId::SquatAscent
		);
	}

	#[test]
	fn settled_wish_does_not_rewrite_heading() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.init_resource::<Time>();
		let body = world.spawn((CharacterController, MoveWish(Vec3::ZERO))).id();
		let visual = world
			.spawn((
				CharacterRoot,
				Transform::IDENTITY,
				CharacterHeading::from_rotation(Quat::IDENTITY),
				PlayerYawOwner::Wish,
				ChildOf(body),
			))
			.id();
		anyhow::ensure!(world.run_system_once(face_wish_yaw).is_ok());
		let before = world
			.entity(visual)
			.get_change_ticks::<Transform>()
			.ok_or_else(|| anyhow::anyhow!("missing transform"))?
			.changed
			.get();
		anyhow::ensure!(world.run_system_once(face_wish_yaw).is_ok());
		let after = world
			.entity(visual)
			.get_change_ticks::<Transform>()
			.ok_or_else(|| anyhow::anyhow!("missing transform"))?
			.changed
			.get();
		anyhow::ensure!(before == after);
		Ok(())
	}
}
