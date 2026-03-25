use bevy::prelude::*;

use crate::local_pathfinding::LocalPathfinding;

use super::utils::{CardinalFanout, GroundWithWallX, OpenGround};

#[test]
fn trace_distance_open_ground_returns_end_and_positive_clearance() {
	let pf = LocalPathfinding::new(CardinalFanout { step: 1.0 }, OpenGround);
	let start = Vec3::ZERO;
	let end = Vec3::new(3.0, 0.0, 0.0);
	let (got, clearance) = pf.trace_distance(start, end).expect("segment should be valid");
	assert!((got - end).length() < 1e-5);
	assert!(clearance > 0.0);
}

#[test]
fn trace_distance_zero_direction_returns_none() {
	let pf = LocalPathfinding::new(CardinalFanout { step: 1.0 }, OpenGround);
	assert!(pf.trace_distance(Vec3::ZERO, Vec3::ZERO).is_none());
}

#[test]
fn trace_distance_wall_shortens_segment_before_wall() {
	let surface = GroundWithWallX { wall_x: 2.0 };
	let mut pf = LocalPathfinding::new(CardinalFanout { step: 1.0 }, surface);
	pf.agent_radius = 0.0;
	pf.trace_depth = 8;
	pf.trace_epsilon = 1e-4;
	pf.collision_response_gain = 1.0;

	let start = Vec3::ZERO;
	let end = Vec3::new(5.0, 0.0, 0.0);
	let (got, _) = pf.trace_distance(start, end).expect("should find stop short of wall");
	assert!(got.x < 2.0, "expected landing before wall at x=2, got {:?}", got);
	assert!((got.y - start.y).abs() < 1e-5 && got.z.abs() < 1e-5);
}
