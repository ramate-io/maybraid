#![doc = include_str!("local_pathfinding/README.md")]

#[cfg(test)]
pub mod testing;

pub mod plugin;

pub use plugin::{respond_to_find_path_requests, FindPath, LocalPathPlan, LocalPathfindingPlugin};

use bevy::prelude::*;

/// A path through a local pathfinding surface.
#[derive(Debug, Clone)]
pub struct LocalPath {
	pub positions: Vec<Vec3>,
}

/// A surface over which we perform local pathfinding.
pub trait LocalPathfindingSurface {
	fn snap_for_local_pathfinding(&self, position: Vec3) -> Vec3;

	/// Returns distance over the surface.
	/// Negative value means collision; absolute value is distance to obstacle.
	fn path_ray_trace_distance(&self, start: Vec3, end: Vec3) -> f32;

	fn local_path_cost(&self, start: Vec3, end: Vec3) -> f32 {
		start.distance(end)
	}
}

/// Generates candidate positions from a given position.
pub trait LocalPathFindingFanout {
	fn local_path_fanout(&self, position: Vec3) -> Vec<Vec3>;
}

/// Internal rollout node
#[derive(Clone)]
struct RolloutNode {
	path: LocalPath,
	cost: f32,
	last_direction: Option<Vec3>,
}

/// Local rollout-based pathfinder.
///
/// When used with [`FindPath`], attach this as a [`Component`] on the same entity (with a
/// [`Transform`]) so each agent can carry its own fanout, surface, and tuning.
#[derive(Clone, Component)]
pub struct LocalPathfinding<F, S>
where
	F: LocalPathFindingFanout + Clone + Send + Sync + 'static,
	S: LocalPathfindingSurface + Clone + Send + Sync + 'static,
{
	pub fanout: F,
	pub surface: S,

	pub depth: usize,

	pub trace_depth: usize,
	pub trace_epsilon: f32,

	pub agent_radius: f32,

	pub collision_response_gain: f32,

	pub weight_goal_cost: f32,
	pub weight_obstacle_repulsion: f32,
	pub weight_path_length: f32,
	pub weight_direction_hysteresis: f32,
	pub weight_progress: f32,
}

impl<F, S> LocalPathfinding<F, S>
where
	F: LocalPathFindingFanout + Clone + Send + Sync + 'static,
	S: LocalPathfindingSurface + Clone + Send + Sync + 'static,
{
	pub fn new(fanout: F, surface: S) -> Self {
		Self {
			fanout,
			surface,
			depth: 3,
			trace_depth: 3,
			trace_epsilon: 0.01,
			agent_radius: 0.5,
			collision_response_gain: 2.0,
			weight_goal_cost: 1.0,
			weight_obstacle_repulsion: 1.0,
			weight_path_length: 1.0,
			weight_direction_hysteresis: 1.0,
			weight_progress: 1.0,
		}
	}

	// --------------------------------------------------
	// Geometry helpers
	// --------------------------------------------------

	fn snap(&self, position: Vec3) -> Vec3 {
		self.surface.snap_for_local_pathfinding(position)
	}

	/// Attempts to validate a ray from `start` to `end` and optionally adjusts the end point.
	///
	/// Contract:
	/// - If the trace returns positive `d`, the segment is passable and `d` is a clearance-like quantity.
	/// - If the trace returns negative `d`, an obstacle was hit, and `-d` is the distance along the ray to that obstacle.
	///
	/// We repeatedly shorten the candidate along the ray direction until we get a positive trace,
	/// or we exhaust `trace_depth`.
	fn trace_distance(&self, start: Vec3, end: Vec3) -> Option<(Vec3, f32)> {
		let dir = (end - start).normalize_or_zero();
		if dir == Vec3::ZERO {
			return None;
		}

		let mut end_candidate = end;

		for _ in 0..self.trace_depth {
			let end_candidate_with_radius = end_candidate + dir * self.agent_radius;

			let d_agent_radius =
				self.surface.path_ray_trace_distance(start, end_candidate_with_radius);

			// Signed clearance
			let clearance = d_agent_radius - self.agent_radius;

			if d_agent_radius > 0.0 {
				return Some((end_candidate, clearance));
			}

			// Penetration depth (how far past contact we went)
			let penetration = (-d_agent_radius).max(0.0);

			// Distance of agent center along the ray
			let candidate_len = start.distance(end_candidate);

			// Over-correct past contact
			// Allow negative lengths so we can flip direction
			let new_len = candidate_len - penetration * self.collision_response_gain;

			let new_candidate = start + dir * new_len;

			if new_candidate.distance(end_candidate) < self.trace_epsilon {
				break;
			}

			end_candidate = new_candidate;
		}

		None
	}

	fn segment_length(&self, start: Vec3, end: Vec3) -> f32 {
		self.surface.local_path_cost(start, end)
	}

	// --------------------------------------------------
	// Cost helpers
	// --------------------------------------------------

	fn obstacle_repulsion(distance: f32) -> f32 {
		let eps = 0.001;
		1.0 / (distance + eps)
	}

	fn goal_cost(&self, position: Vec3, target: Vec3) -> f32 {
		self.weight_goal_cost * position.distance(target)
	}

	fn obstacle_cost(&self, trace_distance: f32) -> f32 {
		self.weight_obstacle_repulsion * Self::obstacle_repulsion(trace_distance)
	}

	fn path_length_cost(&self, length: f32) -> f32 {
		self.weight_path_length * length
	}

	fn progress_cost(&self, from: Vec3, to: Vec3, target: Vec3) -> f32 {
		let before = from.distance(target);
		let after = to.distance(target);
		self.weight_progress * (after - before)
	}

	fn hysteresis_cost(&self, last_dir: Option<Vec3>, new_dir: Vec3) -> f32 {
		if let Some(prev) = last_dir {
			self.weight_direction_hysteresis * (1.0 - prev.dot(new_dir))
		} else {
			0.0
		}
	}

	// --------------------------------------------------
	// Rollout expansion
	// --------------------------------------------------

	fn expand_node(&self, node: &RolloutNode, target: Vec3) -> Vec<RolloutNode> {
		let current = *if let Some(last) = node.path.positions.last() {
			last
		} else {
			return Vec::new();
		};
		let mut children = Vec::new();

		for candidate in self.fanout.local_path_fanout(current) {
			let candidate = self.snap(candidate);

			let (candidate, trace) = match self.trace_distance(current, candidate) {
				Some(d) => d,
				None => continue,
			};

			let segment_len = self.segment_length(current, candidate);
			let dir = (candidate - current).normalize_or_zero();

			let cost = node.cost
				+ self.goal_cost(candidate, target)
				+ self.obstacle_cost(trace)
				+ self.path_length_cost(segment_len)
				+ self.progress_cost(current, candidate, target)
				+ self.hysteresis_cost(node.last_direction, dir);

			let mut path = node.path.clone();
			path.positions.push(candidate);

			children.push(RolloutNode { path, cost, last_direction: Some(dir) });
		}

		children
	}

	// --------------------------------------------------
	// Public API
	// --------------------------------------------------

	/// Finds all partial paths and their accumulated costs.
	pub fn find_partial_paths(&self, start: Vec3, target: Vec3) -> Vec<(LocalPath, f32)> {
		let start = self.snap(start);

		let mut frontier = vec![RolloutNode {
			path: LocalPath { positions: vec![start] },
			cost: 0.0,
			last_direction: None,
		}];

		let mut results = Vec::new();

		for _ in 0..self.depth {
			let mut next_frontier = Vec::new();

			for node in &frontier {
				let children = self.expand_node(node, target);
				for child in children {
					results.push((child.path.clone(), child.cost));
					next_frontier.push(child);
				}
			}

			frontier = next_frontier;
		}

		results
	}
}

#[cfg(test)]
mod simple_tests {
	use bevy::prelude::*;

	use super::testing::utils::{CardinalFanout, OpenGround};
	use super::LocalPath;
	use super::LocalPathfinding;

	#[test]
	fn new_pathfinder_default_depth_is_three() {
		let pf = LocalPathfinding::new(CardinalFanout { step: 1.0 }, OpenGround);
		assert_eq!(pf.depth, 3);
	}

	#[test]
	fn local_path_clones_positions() {
		let a = LocalPath { positions: vec![Vec3::ZERO, Vec3::ONE] };
		let b = a.clone();
		assert_eq!(a.positions, b.positions);
	}
}
