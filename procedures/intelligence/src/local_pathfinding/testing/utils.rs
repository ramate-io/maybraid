//! Shared test doubles for [`crate::local_pathfinding`](crate::local_pathfinding).

use bevy::prelude::*;

use super::super::{LocalPathFindingFanout, LocalPathfindingSurface};

/// Ground plane `z = 0`; rays never report collisions.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpenGround;

impl LocalPathfindingSurface for OpenGround {
	fn snap_for_local_pathfinding(&self, position: Vec3) -> Vec3 {
		Vec3::new(position.x, position.y, 0.0)
	}

	fn path_ray_trace_distance(&self, start: Vec3, end: Vec3) -> f32 {
		start.distance(end)
	}
}

/// `z = 0` snap; an infinite vertical wall at `x = wall_x` blocks segments that cross it.
///
/// Returns the Euclidean segment length when the segment from `start` to `end` does not cross
/// the wall between endpoints. Otherwise returns `-(distance along the ray from start to the
/// crossing)`, matching the negative-hit contract.
#[derive(Clone, Copy, Debug)]
pub struct GroundWithWallX {
	pub wall_x: f32,
}

impl LocalPathfindingSurface for GroundWithWallX {
	fn snap_for_local_pathfinding(&self, position: Vec3) -> Vec3 {
		Vec3::new(position.x, position.y, 0.0)
	}

	fn path_ray_trace_distance(&self, start: Vec3, end: Vec3) -> f32 {
		let d = end - start;
		let len = d.length();
		if len < 1e-12 {
			return len;
		}
		if d.x.abs() < 1e-6 {
			return len;
		}

		let u = (self.wall_x - start.x) / d.x;
		if u <= 0.0 || u >= 1.0 {
			return len;
		}

		-(u * len)
	}
}

/// Four axis-aligned neighbors at distance `step` in the XY plane.
#[derive(Clone, Copy, Debug)]
pub struct CardinalFanout {
	pub step: f32,
}

impl LocalPathFindingFanout for CardinalFanout {
	fn local_path_fanout(&self, position: Vec3) -> Vec<Vec3> {
		let s = self.step;
		vec![
			position + Vec3::X * s,
			position - Vec3::X * s,
			position + Vec3::Y * s,
			position - Vec3::Y * s,
		]
	}
}

/// Always returns `points` regardless of current position (useful for deterministic tests).
#[derive(Clone, Debug)]
pub struct FixedFanout {
	pub points: Vec<Vec3>,
}

impl LocalPathFindingFanout for FixedFanout {
	fn local_path_fanout(&self, _position: Vec3) -> Vec<Vec3> {
		self.points.clone()
	}
}
