//! Wish facing and walk/run/jump clips.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use characters::{
	AnimClip, AnimMailbox, AnimProgress, AnimRef, AnimRefRoot, CharacterHeading, CharacterMembers,
	CharacterRig, CharacterRigRole, CharacterRoot, JumpParams, RigSkeletonKind,
};

use crate::body::{CharacterController, Grounded, Jumping, MoveWish, LEAP_SPEED};
use crate::identity::PlayerYawOwner;
use crate::stance::{CharacterStance, StanceKind};

pub(crate) const WALK_SPEED: f32 = 1.0;
const STAND_WALK_ENTER: f32 = 1.12;
const STAND_WALK_EXIT: f32 = 0.88;
const STAND_RUN_EXIT: f32 = 7.1;

/// Hysteresis band for humanoid stand locomotion clips (still / walk / run).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StandLocomotionGait {
	band: StandLocomotionBand,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StandLocomotionBand {
	#[default]
	Still,
	Walk,
	Run,
}

impl StandLocomotionBand {
	fn advance(self, speed: f32) -> Self {
		match self {
			Self::Still => {
				if speed > LEAP_SPEED {
					Self::Run
				} else if speed > STAND_WALK_ENTER {
					Self::Walk
				} else {
					Self::Still
				}
			}
			Self::Walk => {
				if speed > LEAP_SPEED {
					Self::Run
				} else if speed < STAND_WALK_EXIT {
					Self::Still
				} else {
					Self::Walk
				}
			}
			Self::Run => {
				if speed <= STAND_RUN_EXIT {
					if speed < STAND_WALK_EXIT {
						Self::Still
					} else {
						Self::Walk
					}
				} else {
					Self::Run
				}
			}
		}
	}

	fn clip(self) -> AnimClip {
		match self {
			Self::Still => AnimClip::still(),
			Self::Walk => AnimClip::walk(),
			Self::Run => AnimClip::run(),
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
				&& jumping.is_none()
				&& stance.squat_settled()
				&& matches!(stance.kind, StanceKind::Stand)
			{
				let band = gait.get(member).map(|g| g.band).unwrap_or_default().advance(speed);
				commands.entity(member).insert(StandLocomotionGait { band });
				Some(band)
			} else {
				None
			};
			let current = anims.get(member).ok();
			let clip = locomotion_clip(rig.skeleton, jumping, &stance, speed, stand_band);
			let clip = if rig.skeleton == RigSkeletonKind::Quadruped
				&& grounded && jumping.is_none()
				&& stance.squat_settled()
				&& matches!(stance.kind, StanceKind::Stand)
				&& clip.id() == characters::AnimId::Still
				&& current.is_some_and(|(root, mailbox)| {
					root.0
						.clip
						.id()
						.needs_quadruped_run_stop(mailbox.map_or(0.0, AnimMailbox::clip_progress))
				}) {
				AnimClip::quadruped_run_stop()
			} else {
				clip
			};
			let desired = AnimRef::new(clip);
			let needs = current.is_none_or(|(root, _)| root.0 != desired);
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
	stand_gait: Option<StandLocomotionBand>,
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
					StanceKind::Stand => stand_gait
						.map(StandLocomotionBand::clip)
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
	if speed > LEAP_SPEED {
		AnimClip::run()
	} else if speed > WALK_SPEED {
		AnimClip::walk()
	} else {
		AnimClip::still()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::body::{JumpPhase, JOG_SPEED, MOVE_SPEED};
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
			schedule.run(&mut world);
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
	fn clip_follows_the_cap() {
		let stance = CharacterStance::settled(StanceKind::Stand);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, JOG_SPEED, None).id(),
			AnimId::Walk
		);
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, MOVE_SPEED, None).id(),
			AnimId::Run
		);
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

	/// Counts clip-id changes when speed samples cross the direct `WALK_SPEED` threshold.
	fn direct_stand_clip_changes(speeds: &[f32]) -> usize {
		let mut changes = 0usize;
		let mut last: Option<AnimId> = None;
		for &speed in speeds {
			let id = stand_locomotion_clip_direct(speed).id();
			if last != Some(id) {
				changes += 1;
				last = Some(id);
			}
		}
		changes
	}

	fn gait_stand_clip_changes(speeds: &[f32]) -> usize {
		let mut band = StandLocomotionBand::Still;
		let stance = CharacterStance::settled(StanceKind::Stand);
		let mut changes = 0usize;
		let mut last: Option<AnimId> = None;
		for &speed in speeds {
			band = band.advance(speed);
			let id =
				locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, speed, Some(band)).id();
			if last != Some(id) {
				changes += 1;
				last = Some(id);
			}
		}
		changes
	}

	#[test]
	fn direct_stand_threshold_noise_restarts_still_walk_transitions() {
		let noise = [0.99, 1.01, 0.99, 1.01, 0.99, 1.01];
		let changes = direct_stand_clip_changes(&noise);
		assert!(
			changes >= 5,
			"direct thresholds at WALK_SPEED=1.0 should restart still/walk on ±0.01 m/s noise, got {changes} changes"
		);
	}

	#[test]
	fn gait_hysteresis_suppresses_still_walk_noise_restarts() {
		let noise = [1.05, 0.95, 1.05, 0.95, 1.05, 0.95];
		let changes = gait_stand_clip_changes(&noise);
		assert_eq!(
			changes, 1,
			"hysteresis should hold still/walk across ±0.05 m/s flutter around 1.0 m/s"
		);
	}

	#[test]
	fn gait_still_crosses_to_walk_on_intentional_speed() {
		let speeds = [0.5, 0.5, 1.25, 1.25];
		let changes = gait_stand_clip_changes(&speeds);
		assert_eq!(changes, 2, "intentional crossing should still switch still→walk promptly");
	}

	#[test]
	fn direct_run_walk_boundary_can_chatter_at_leap_speed() {
		let noise = [7.49, 7.51, 7.49, 7.51, 7.49, 7.51];
		let changes = direct_stand_clip_changes(&noise);
		assert!(
			changes >= 5,
			"direct thresholds at LEAP_SPEED=7.5 can restart walk/run on ±0.01 m/s noise"
		);
	}

	#[test]
	fn typical_jog_and_sprint_speeds_stay_off_run_walk_boundary() {
		assert_eq!(stand_locomotion_clip_direct(JOG_SPEED).id(), AnimId::Walk);
		assert_eq!(stand_locomotion_clip_direct(MOVE_SPEED).id(), AnimId::Run);
	}

	#[test]
	fn leaving_squat_uses_descent_until_blend_reaches_zero() {
		let stance = CharacterStance { kind: StanceKind::Stand, blend: 0.5 };
		assert_eq!(
			locomotion_clip(RigSkeletonKind::Humanoid, None, &stance, 0.0, None).id(),
			AnimId::SquatDescent
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
