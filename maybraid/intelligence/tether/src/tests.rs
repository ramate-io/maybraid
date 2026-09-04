use bevy::prelude::*;
use movement_intelligence::MovementObjective;

use crate::memory::TetherMemory;
use crate::objective::{ring_point, TetherObjective};
use crate::user::{TetherAction, TetherIntelligenceUser};

fn subject() -> Entity {
	Entity::from_bits(7)
}

#[test]
fn tether_remaining_is_leash_overflow() -> anyhow::Result<()> {
	let objective = TetherObjective::Tether(subject(), 8.0);
	assert!((objective.remaining(Vec3::ZERO, Vec3::X * 20.0) - 12.0).abs() < 1e-4);
	assert!(objective.remaining(Vec3::ZERO, Vec3::X * 4.0) < 1e-4);
	Ok(())
}

#[test]
fn stalk_remaining_is_distance_to_the_ring() -> anyhow::Result<()> {
	let objective = TetherObjective::Stalk(subject(), 10.0);
	assert!((objective.remaining(Vec3::ZERO, Vec3::X * 4.0) - 6.0).abs() < 1e-4);
	assert!((objective.remaining(Vec3::ZERO, Vec3::X * 16.0) - 6.0).abs() < 1e-4);
	assert!(objective.remaining(Vec3::ZERO, Vec3::X * 10.0) < 1e-4);
	Ok(())
}

#[test]
fn stalk_routes_to_the_near_ring_point() -> anyhow::Result<()> {
	let objective = TetherObjective::Stalk(subject(), 10.0);
	let from = Vec3::X * 40.0;
	let subject_at = Vec3::ZERO;
	let point = objective.route_point(from, subject_at);
	assert!((ring_point(from, subject_at, 10.0) - point).length() < 1e-4);
	assert!((point.x - 10.0).abs() < 1e-4);
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
	let mut user = TetherIntelligenceUser::new(TetherObjective::Stalk(subject(), 8.0))
		.with_horizon(20.0)
		.with_stuck_timeout(0.0);
	let mut memory = TetherMemory::new(subject());
	let action = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 14.0, 0.25, 1.0);
	assert!(matches!(action, TetherAction::Local(MovementObjective::EdgeOf(_))));
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
fn disabled_grant_holds_once_then_idles() -> anyhow::Result<()> {
	let mut user = TetherIntelligenceUser::new(TetherObjective::Tether(subject(), 4.0))
		.with_horizon(8.0)
		.with_stuck_timeout(0.0);
	let mut memory = TetherMemory::new(subject());
	let _ = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 40.0, 0.25, 1.0);
	user.enabled = false;
	let release = user.evaluate(&mut memory, Vec3::ZERO, Vec3::X * 40.0, 0.25, 2.0);
	assert!(matches!(release, TetherAction::Hold));
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
