use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use movement_intelligence::{MovementIntelligence, MovementLocation, MovementObjective};

use crate::memory::TetherMemory;
use crate::objective::{ring_point, StalkRadii, TetherObjective};
use crate::plugin::TetherPlugin;
use crate::user::{Tether, TetherAction, TetherIntelligenceUser};

fn subject() -> Entity {
	Entity::from_bits(7)
}

#[test]
fn lock_standoff_cycles_slot_radii() -> anyhow::Result<()> {
	assert_eq!(TetherObjective::lock_standoff(0), 6.0);
	assert_eq!(TetherObjective::lock_standoff(1), 8.0);
	assert_eq!(TetherObjective::lock_standoff(2), 10.0);
	assert_eq!(TetherObjective::lock_standoff(3), 6.0);
	let idle = TetherObjective::Tether(subject(), 24.0);
	assert!(matches!(
		idle.with_lock_standoff(1),
		TetherObjective::Tether(_, radius) if (radius - 8.0).abs() < 1e-5
	));
	Ok(())
}

#[test]
fn stalk_radii_clamp_to_a_valid_annulus() -> anyhow::Result<()> {
	let radii = StalkRadii::new(-2.0, -4.0);
	assert_eq!(radii.without(), 0.0);
	assert_eq!(radii.within(), 0.0);
	let radii = StalkRadii::new(12.0, 8.0);
	assert_eq!(radii.without(), 12.0);
	assert_eq!(radii.within(), 12.0);
	Ok(())
}

#[test]
fn tether_remaining_is_leash_overflow() -> anyhow::Result<()> {
	let objective = TetherObjective::Tether(subject(), 8.0);
	assert!((objective.remaining(Vec3::ZERO, Vec3::X * 20.0) - 12.0).abs() < 1e-4);
	assert!(objective.remaining(Vec3::ZERO, Vec3::X * 4.0) < 1e-4);
	Ok(())
}

#[test]
fn stalk_remaining_is_distance_to_the_allowed_annulus() -> anyhow::Result<()> {
	let objective = TetherObjective::Stalk(subject(), StalkRadii::new(8.0, 12.0));
	assert!((objective.remaining(Vec3::ZERO, Vec3::X * 4.0) - 4.0).abs() < 1e-4);
	assert!((objective.remaining(Vec3::ZERO, Vec3::X * 16.0) - 4.0).abs() < 1e-4);
	assert!(objective.remaining(Vec3::ZERO, Vec3::X * 8.0) < 1e-4);
	assert!(objective.remaining(Vec3::ZERO, Vec3::X * 10.0) < 1e-4);
	assert!(objective.remaining(Vec3::ZERO, Vec3::X * 12.0) < 1e-4);
	Ok(())
}

#[test]
fn stalk_routes_to_the_nearest_annulus_boundary() -> anyhow::Result<()> {
	let objective = TetherObjective::Stalk(subject(), StalkRadii::new(8.0, 12.0));
	let subject_at = Vec3::ZERO;
	let outside = objective.route_point(Vec3::X * 40.0, subject_at);
	assert!((ring_point(Vec3::X * 40.0, subject_at, 12.0) - outside).length() < 1e-4);
	assert!((outside.x - 12.0).abs() < 1e-4);
	let inside = objective.route_point(Vec3::X * 2.0, subject_at);
	assert!((ring_point(Vec3::X * 2.0, subject_at, 8.0) - inside).length() < 1e-4);
	assert!((inside.x - 8.0).abs() < 1e-4);
	Ok(())
}

#[test]
fn close_tether_writes_reach() -> anyhow::Result<()> {
	let mut user = TetherIntelligenceUser::new(TetherObjective::Tether(subject(), 4.0))
		.with_horizon(20.0)
		.with_stuck_timeout(0.0);
	let mut memory = TetherMemory::new(subject());
	let action = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 10.0, 0.25, 1.0);
	assert!(matches!(action, TetherAction::Local(MovementObjective::Reach(_))));
	assert!(!memory.satisfied);
	Ok(())
}

#[test]
fn far_tether_writes_a_route() -> anyhow::Result<()> {
	let mut user = TetherIntelligenceUser::new(TetherObjective::Tether(subject(), 4.0))
		.with_horizon(8.0)
		.with_stuck_timeout(0.0);
	let mut memory = TetherMemory::new(subject());
	let action = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 40.0, 0.25, 1.0);
	assert!(matches!(action, TetherAction::Route(point) if (point.x - 40.0).abs() < 1e-4));
	Ok(())
}

#[test]
fn close_stalk_writes_edge_of() -> anyhow::Result<()> {
	let mut user =
		TetherIntelligenceUser::new(TetherObjective::Stalk(subject(), StalkRadii::new(8.0, 12.0)))
			.with_horizon(20.0)
			.with_stuck_timeout(0.0);
	let mut memory = TetherMemory::new(subject());
	let too_far = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 14.0, 0.25, 1.0);
	assert!(matches!(
		too_far,
		TetherAction::Local(MovementObjective::EdgeOf(location))
			if (location.radius - 12.0).abs() < 1e-4
	));
	let too_close = user.evaluate(&mut memory, Vec3::X * 12.0, Vec3::X * 14.0, 0.25, 2.0);
	assert!(matches!(
		too_close,
		TetherAction::Local(MovementObjective::EdgeOf(location))
			if (location.radius - 8.0).abs() < 1e-4
	));
	Ok(())
}

#[test]
fn hysteresis_keeps_satisfied_until_added_radius() -> anyhow::Result<()> {
	let mut user = TetherIntelligenceUser::new(TetherObjective::Tether(subject(), 4.0))
		.with_added_radius(3.0)
		.with_horizon(20.0)
		.with_stuck_timeout(0.0);
	let mut memory = TetherMemory::new(subject());
	let inside = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 3.0, 0.25, 1.0);
	assert!(matches!(inside, TetherAction::Hold | TetherAction::None));
	assert!(memory.satisfied);
	let slack = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 6.5, 0.25, 2.0);
	assert!(memory.satisfied);
	assert!(matches!(slack, TetherAction::None));
	let pulled = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 10.0, 0.25, 3.0);
	assert!(!memory.satisfied);
	assert!(matches!(pulled, TetherAction::Local(_) | TetherAction::Route(_)));
	Ok(())
}

#[test]
fn reclaim_stolen_objective_overwrites_firearm_vantage() -> anyhow::Result<()> {
	use crate::plugin::reclaim_stolen_objective;

	let subject = subject();
	let objective = TetherObjective::Tether(subject, 24.0);
	let from = Vec3::X * 26.0;
	let anchor = Vec3::ZERO;
	let stolen = MovementObjective::VantageOn {
		location: MovementLocation::new(Vec3::X * 50.0, 14.0),
		hide_weight: 0.0,
		sightline_weight: 1.0,
	};
	let reclaimed = reclaim_stolen_objective(
		TetherAction::None,
		true,
		false,
		2.0,
		28.0,
		objective,
		from,
		anchor,
		stolen,
	);
	assert!(matches!(
		reclaimed,
		TetherAction::Local(MovementObjective::Reach(location))
			if (location.radius - 24.0).abs() < 1e-4
	));
	let already_ours = MovementObjective::Reach(MovementLocation::new(anchor, 24.0));
	let yielded = reclaim_stolen_objective(
		TetherAction::None,
		true,
		false,
		2.0,
		28.0,
		objective,
		from,
		anchor,
		already_ours,
	);
	assert!(matches!(yielded, TetherAction::None));
	let satisfied = reclaim_stolen_objective(
		TetherAction::None,
		true,
		true,
		0.0,
		28.0,
		objective,
		from,
		anchor,
		stolen,
	);
	assert!(matches!(satisfied, TetherAction::None));
	let disabled = reclaim_stolen_objective(
		TetherAction::None,
		false,
		false,
		2.0,
		28.0,
		objective,
		from,
		anchor,
		stolen,
	);
	assert!(matches!(disabled, TetherAction::None));
	Ok(())
}

#[test]
fn progress_does_not_replan() -> anyhow::Result<()> {
	let mut user = TetherIntelligenceUser::new(TetherObjective::Tether(subject(), 4.0))
		.with_horizon(8.0)
		.with_stuck_timeout(10.0);
	let mut memory = TetherMemory::new(subject());
	let first = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 40.0, 0.25, 1.0);
	assert!(matches!(first, TetherAction::Route(_)));
	let closer = user.evaluate(&mut memory, Vec3::X * 10.0, Vec3::X * 40.0, 0.25, 2.0);
	assert!(matches!(closer, TetherAction::None));
	Ok(())
}

#[test]
fn disabled_grant_does_not_hold() -> anyhow::Result<()> {
	let mut user = TetherIntelligenceUser::new(TetherObjective::Tether(subject(), 4.0))
		.with_horizon(8.0)
		.with_stuck_timeout(0.0);
	let mut memory = TetherMemory::new(subject());
	let writing = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 40.0, 0.25, 1.0);
	assert!(matches!(writing, TetherAction::Route(_)));
	user.enabled = false;
	let release = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 40.0, 0.25, 2.0);
	assert!(matches!(release, TetherAction::None));
	let idle = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 40.0, 0.25, 3.0);
	assert!(matches!(idle, TetherAction::None));
	Ok(())
}

#[test]
fn disabled_grant_writer_leaves_a_flee_objective() -> anyhow::Result<()> {
	use std::time::Duration;

	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TransformPlugin, TetherPlugin))
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(0.25)));

	let anchor = app
		.world_mut()
		.spawn((Tether, Transform::IDENTITY, GlobalTransform::IDENTITY))
		.id();
	let mut user = TetherIntelligenceUser::new(TetherObjective::Tether(anchor, 4.0))
		.with_horizon(20.0)
		.with_stuck_timeout(0.0);
	user.enabled = true;
	let outside = Transform::from_translation(Vec3::X * 20.0);
	let actor = app
		.world_mut()
		.spawn((
			outside,
			GlobalTransform::from(outside),
			user,
			TetherMemory {
				subject: anchor,
				satisfied: false,
				remaining: 16.0,
				last_checked_at: 0.0,
			},
			MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
				Vec3::ZERO,
				0.4,
			))),
		))
		.id();

	app.update();
	let pulled = app.world().get::<MovementIntelligence>(actor).expect("movement").objective;
	anyhow::ensure!(
		matches!(pulled, MovementObjective::Reach(_)),
		"enabled tether should pull an actor outside its leash, got {pulled:?}"
	);

	app.world_mut()
		.get_mut::<TetherIntelligenceUser>(actor)
		.expect("tether user")
		.enabled = false;
	let flee = MovementObjective::FleeFrom(MovementLocation::new(Vec3::X * 50.0, 16.0));
	app.world_mut()
		.get_mut::<MovementIntelligence>(actor)
		.expect("movement")
		.objective = flee;

	app.update();
	let after = app.world().get::<MovementIntelligence>(actor).expect("movement").objective;
	anyhow::ensure!(
		matches!(after, MovementObjective::FleeFrom(_)),
		"disabled tether must leave a fresh flee objective, got {after:?}"
	);
	Ok(())
}

#[test]
fn memory_is_independent_of_the_user() -> anyhow::Result<()> {
	let mut user = TetherIntelligenceUser::new(TetherObjective::Tether(subject(), 4.0))
		.with_horizon(20.0)
		.with_stuck_timeout(0.0);
	let mut memory = TetherMemory::new(subject());
	let _ = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 2.0, 0.25, 1.0);
	assert!(memory.satisfied);
	drop(user);
	assert!(memory.satisfied);
	assert_eq!(memory.subject, subject());
	Ok(())
}
