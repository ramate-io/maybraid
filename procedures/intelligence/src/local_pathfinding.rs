use bevy::prelude::*;

/// A path through a local pathfinding surface.
#[derive(Debug, Clone)]
pub struct LocalPath {
	positions: Vec<Vec3>,
}

/// Memory for tracking generated paths with helpers for eviction.
#[derive(Debug, Clone)]
pub struct PathMemory {
	paths: Vec<LocalPath>,
}

/// A surface over which we perform local pathfinding.
pub trait LocalPathfindingSurface {
	/// Snaps to the nearest reasonable position.
	///
	/// This is used prior to the ray trace.
	fn snap_for_local_pathfinding(&self, position: Vec3) -> Vec3;

	/// Gives the distance over the surface from one position to another.
	///
	/// If an impassable obstacle is encountered, the distance should be negative distance to that obstacle.
	/// For the most part, this will require tracing a ray from start to end and checking for obstacles.
	fn path_ray_trace_distance(&self, start: Vec3, end: Vec3) -> bool;

	/// Gives the cost for a ray path from one position to another.
	///
	/// By default, this is the Euclidean distance.
	fn local_path_cost(&self, start: Vec3, end: Vec3) -> f32 {
		start.distance(end)
	}
}

pub trait LocalPathFindingFanout {
	/// Gives initial position from a point to fan out from.
	fn local_path_fanout(&self, position: Vec3) -> Vec<Vec3>;
}

pub struct LocalPathfinding<F: LocalPathFindingFanout, S: LocalPathfindingSurface> {
	fanout: F,
	surface: S,
	depth: usize,
	weight_goal_cost: f32,
	weight_obstacle_repulsion: f32,
	weight_path_length: f32,
	weight_direction_hysteresis: f32,
	weight_progress: f32,
}

impl<F: LocalPathFindingFanout, S: LocalPathfindingSurface> LocalPathfinding<F, S> {
	pub fn new(fanout: F, surface: S) -> Self {
		Self {
			fanout,
			surface,
			depth: 3,
			weight_goal_cost: 1.0,
			weight_obstacle_repulsion: 1.0,
			weight_path_length: 1.0,
			weight_direction_hysteresis: 1.0,
			weight_progress: 1.0,
		}
	}

	/// Snaps a position to the nearest reasonable position on the surface.
	pub fn snap(&self, position: Vec3) -> Vec3 {
		self.surface.snap_for_local_pathfinding(position)
	}

	/// Traces a ray from start to end and checks for obstacles.
	pub fn path_ray_trace_distance(&self, start: Vec3, end: Vec3) -> bool {
		self.surface.path_ray_trace_distance(start, end)
	}

	/// Gives the cost for a local path from one position to another.
	pub fn local_path_cost(&self, start: Vec3, end: Vec3) -> f32 {
		self.surface.local_path_cost(start, end)
	}

	pub fn find_paths(&self, position: Vec3, target: Vec3) -> Vec<LocalPath> {
		todo!()
	}
}
