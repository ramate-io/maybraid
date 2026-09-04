use bevy::prelude::*;
use movement_intelligence::MovementObjective;

use crate::memory::TetherMemory;
use crate::objective::{StalkRadii, TetherObjective, ring_point};
use crate::user::{TetherAction, TetherIntelligenceUser};

fn subject() -> Entity {
	Entity::from_bits(7)
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
