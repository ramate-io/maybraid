use bevy::prelude::*;

use crate::local_pathfinding::LocalPathfinding;

use super::utils::{CardinalFanout, FixedFanout, OpenGround};

#[test]
fn find_partial_paths_depth_zero_yields_no_results() {
	let pf = LocalPathfinding::new(CardinalFanout { step: 1.0 }, OpenGround);
	let mut pf = pf;
	pf.depth = 0;
	let out = pf.find_partial_paths(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
	assert!(out.is_empty());
}

#[test]
fn find_partial_paths_one_layer_emits_one_path_per_fanout_child() {
	let pf = LocalPathfinding::new(CardinalFanout { step: 1.0 }, OpenGround);
	let mut pf = pf;
	pf.depth = 1;
	let target = Vec3::new(10.0, 0.0, 0.0);
	let out = pf.find_partial_paths(Vec3::ZERO, target);
	assert_eq!(out.len(), 4);
	for (path, cost) in &out {
		assert_eq!(path.positions.len(), 2);
		assert!(cost.is_finite());
	}
}

#[test]
fn find_partial_paths_snaps_start_to_ground() {
	let pf = LocalPathfinding::new(
		FixedFanout {
			points: vec![Vec3::new(1.0, 0.0, 0.0)],
		},
		OpenGround,
	);
	let mut pf = pf;
	pf.depth = 1;
	let out = pf.find_partial_paths(Vec3::new(0.0, 0.0, 9.0), Vec3::ZERO);
	assert_eq!(out.len(), 1);
	let (path, _) = &out[0];
	assert_eq!(path.positions[0], Vec3::ZERO);
}

#[test]
fn find_partial_paths_accumulates_each_rollout_layer() {
	let pf = LocalPathfinding::new(CardinalFanout { step: 1.0 }, OpenGround);
	let mut pf = pf;
	pf.depth = 3;
	let out = pf.find_partial_paths(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
	assert!(!out.is_empty());
	let lengths: Vec<usize> = out.iter().map(|(p, _)| p.positions.len()).collect();
	assert!(lengths.iter().any(|&n| n == pf.depth + 1));
	assert!(lengths.iter().all(|&n| (2..=pf.depth + 1).contains(&n)));
}
