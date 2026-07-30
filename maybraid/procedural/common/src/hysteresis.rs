//! Bounded hysteresis path / graph construction (RFC-127-style walks).
//!
//! These utilities operate inside a 2D axis-aligned [`Bounds2`]. They do **not**
//! use generation-"cell" terminology; cellular generation (`OriginCell`, LOD
//! schemes) stays in lod / `-models` crates.

use bevy_math::Vec2;
use std::collections::VecDeque;
use std::f32::consts::TAU;

/// Axis-aligned rectangle in a 2D plane (typically world XZ mapped to `Vec2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds2 {
	pub min: Vec2,
	pub max: Vec2,
}

impl Bounds2 {
	pub fn new(min: Vec2, max: Vec2) -> Self {
		Self { min, max }
	}

	pub fn from_xz(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Self {
		Self::new(Vec2::new(min_x, min_z), Vec2::new(max_x, max_z))
	}

	pub fn contains(&self, p: Vec2) -> bool {
		p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
	}

	pub fn project(&self, p: Vec2) -> Vec2 {
		Vec2::new(p.x.clamp(self.min.x, self.max.x), p.y.clamp(self.min.y, self.max.y))
	}

	pub fn center(&self) -> Vec2 {
		(self.min + self.max) * 0.5
	}

	pub fn extent(&self) -> Vec2 {
		self.max - self.min
	}
}

/// Walk parameters for bounded hysteresis construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HysteresisConfig {
	pub max_segments: usize,
	pub step_len: f32,
	pub snap_radius: f32,
	pub connect_radius: f32,
	pub min_progress: f32,
	/// Blend toward previous heading in `[0, 1]` (higher = stickier).
	pub hysteresis: f32,
	pub max_turn_radians: f32,
}

impl Default for HysteresisConfig {
	fn default() -> Self {
		Self {
			max_segments: 24,
			step_len: 24.0,
			snap_radius: 18.0,
			connect_radius: 40.0,
			min_progress: 1.0,
			hysteresis: 0.55,
			max_turn_radians: 0.65,
		}
	}
}

/// Deterministic unit samples in `[0, 1)` from a seed + salt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeededHash {
	pub seed: u32,
}

impl SeededHash {
	pub fn new(seed: u32) -> Self {
		Self { seed }
	}

	pub fn unit(&self, salt: u32) -> f32 {
		let mut x =
			self.seed.wrapping_mul(0x9E37_79B9).wrapping_add(salt.wrapping_mul(0x85EB_CA6B));
		x ^= x >> 16;
		x = x.wrapping_mul(0x7FEB_352D);
		x ^= x >> 15;
		(x as f32) / (u32::MAX as f32)
	}

	pub fn unit_i32(&self, a: i32, b: i32) -> f32 {
		self.unit((a as u32).wrapping_mul(73856093) ^ (b as u32).wrapping_mul(19349663))
	}
}

/// Directed graph of points grown by hysteresis walks (max out-degree 1..=4).
#[derive(Debug, Clone, PartialEq)]
pub struct HysteresisGraph {
	pub nodes: Vec<Vec2>,
	pub children: Vec<Vec<usize>>,
}

impl HysteresisGraph {
	/// Degree-1: a single hysteresis path from `start` toward `end` inside `bounds`.
	pub fn degree1(
		bounds: Bounds2,
		seed: u32,
		start: Vec2,
		end: Vec2,
		config: &HysteresisConfig,
	) -> Self {
		Self::from_path(Self::walk_path(bounds, seed, start, end, config))
	}

	/// Degree-2: primary path plus one side spur from each interior node (capped).
	pub fn degree2(
		bounds: Bounds2,
		seed: u32,
		start: Vec2,
		end: Vec2,
		config: &HysteresisConfig,
	) -> Self {
		Self::branched(2, bounds, seed, start, end, config)
	}

	/// Degree-3: primary path plus up to two side spurs per interior node.
	pub fn degree3(
		bounds: Bounds2,
		seed: u32,
		start: Vec2,
		end: Vec2,
		config: &HysteresisConfig,
	) -> Self {
		Self::branched(3, bounds, seed, start, end, config)
	}

	/// Degree-4: primary path plus up to three side spurs per interior node.
	pub fn degree4(
		bounds: Bounds2,
		seed: u32,
		start: Vec2,
		end: Vec2,
		config: &HysteresisConfig,
	) -> Self {
		Self::branched(4, bounds, seed, start, end, config)
	}

	/// Build with `degree` in `1..=4`.
	pub fn with_degree(
		degree: u8,
		bounds: Bounds2,
		seed: u32,
		start: Vec2,
		end: Vec2,
		config: &HysteresisConfig,
	) -> Self {
		match degree.max(1).min(4) {
			1 => Self::degree1(bounds, seed, start, end, config),
			2 => Self::degree2(bounds, seed, start, end, config),
			3 => Self::degree3(bounds, seed, start, end, config),
			_ => Self::degree4(bounds, seed, start, end, config),
		}
	}

	/// Leaf / tip positions (nodes with no children), useful as stamp anchors.
	pub fn tip_points(&self) -> Vec<Vec2> {
		self.nodes
			.iter()
			.enumerate()
			.filter(|(i, _)| self.children.get(*i).is_none_or(|c| c.is_empty()))
			.map(|(_, p)| *p)
			.collect()
	}

	/// `count` degree-1 spurs from `bounds.center()` toward evenly spaced radial targets.
	pub fn radial_tips(
		bounds: Bounds2,
		seed: u32,
		count: usize,
		config: &HysteresisConfig,
	) -> Vec<Vec2> {
		let center = bounds.center();
		let hash = SeededHash::new(seed);
		let mut tips = Vec::with_capacity(count);
		for i in 0..count {
			let angle = (i as f32 + 0.37) * TAU / count.max(1) as f32;
			let radius = 0.25 + 0.35 * hash.unit(i as u32 + 11);
			let end = bounds.project(
				center
					+ Vec2::new(angle.cos(), angle.sin()) * radius * bounds.extent().min_element(),
			);
			let path = Self::degree1(bounds, seed.wrapping_add(i as u32 * 17), center, end, config);
			if let Some(p) = path.nodes.last().copied() {
				tips.push(p);
			}
		}
		tips
	}

	/// Polyline of the degree-1 spine when the graph is a path; otherwise root→first-child chain.
	pub fn primary_polyline(&self) -> Vec<Vec2> {
		if self.nodes.is_empty() {
			return Vec::new();
		}
		let mut out = vec![self.nodes[0]];
		let mut idx = 0usize;
		while let Some(child) = self.children.get(idx).and_then(|c| c.first()).copied() {
			out.push(self.nodes[child]);
			idx = child;
		}
		out
	}

	fn from_path(path: Vec<Vec2>) -> Self {
		let n = path.len();
		let mut children = vec![Vec::new(); n];
		for i in 0..n.saturating_sub(1) {
			children[i].push(i + 1);
		}
		Self { nodes: path, children }
	}

	fn branched(
		degree: u8,
		bounds: Bounds2,
		seed: u32,
		start: Vec2,
		end: Vec2,
		config: &HysteresisConfig,
	) -> Self {
		let path = Self::walk_path(bounds, seed, start, end, config);
		let mut graph = Self::from_path(path);
		if degree <= 1 || graph.nodes.len() < 3 {
			return graph;
		}

		let side_budget = (degree - 1) as usize;
		let hash = SeededHash::new(seed);
		let mut queue = VecDeque::new();
		for i in 1..graph.nodes.len().saturating_sub(1) {
			queue.push_back(i);
		}

		let mut spur_salt = 0u32;
		while let Some(parent) = queue.pop_front() {
			let existing = graph.children[parent].len();
			// Keep the primary forward edge; add up to `side_budget` side children.
			let primary_forward = existing.min(1);
			let room = side_budget.saturating_sub(existing.saturating_sub(primary_forward));
			if room == 0 {
				continue;
			}
			let origin = graph.nodes[parent];
			for _ in 0..room {
				spur_salt = spur_salt.wrapping_add(1);
				let angle = hash.unit(spur_salt) * TAU;
				let reach = 0.2 + 0.35 * hash.unit(spur_salt.wrapping_add(9));
				let tip = bounds.project(
					origin
						+ Vec2::new(angle.cos(), angle.sin())
							* reach * bounds.extent().min_element(),
				);
				let spur = Self::walk_path(
					bounds,
					seed.wrapping_add(spur_salt),
					origin,
					tip,
					&HysteresisConfig {
						max_segments: 8,
						step_len: config.step_len * 0.6,
						..*config
					},
				);
				if spur.len() < 2 {
					continue;
				}
				// Attach spur excluding the duplicated origin.
				let mut parent_idx = parent;
				for &p in spur.iter().skip(1) {
					let child_idx = graph.nodes.len();
					graph.nodes.push(p);
					graph.children.push(Vec::new());
					graph.children[parent_idx].push(child_idx);
					parent_idx = child_idx;
				}
			}
		}
		graph
	}

	fn walk_path(
		bounds: Bounds2,
		seed: u32,
		start: Vec2,
		end: Vec2,
		config: &HysteresisConfig,
	) -> Vec<Vec2> {
		let mut p = bounds.project(start);
		let end = bounds.project(end);
		let mut dir_prev = (end - p).normalize_or_zero();
		if dir_prev.length_squared() < 1e-8 {
			return vec![p];
		}
		let mut out = vec![p];
		let hash = SeededHash::new(seed);

		for k in 0..config.max_segments {
			let to_end = end - p;
			if to_end.length() <= config.snap_radius {
				out.push(end);
				break;
			}

			let dir_goal = to_end.normalize_or_zero();
			let theta = (hash.unit(k as u32) * 2.0 - 1.0) * config.max_turn_radians;
			let blended = lerp_dir(dir_goal, dir_prev, config.hysteresis);
			let dir = rotate(blended, theta);

			let mut q = p + dir * config.step_len;
			if !bounds.contains(q) {
				q = bounds.project(q);
				if q.distance(p) < config.min_progress {
					break;
				}
			}

			out.push(q);
			p = q;
			dir_prev = dir;
		}

		if out.last().is_some_and(|last| last.distance(end) <= config.connect_radius) {
			out.push(end);
		}
		out
	}
}

fn lerp_dir(a: Vec2, b: Vec2, t: f32) -> Vec2 {
	(a * (1.0 - t) + b * t).normalize_or_zero()
}

fn rotate(v: Vec2, radians: f32) -> Vec2 {
	let (s, c) = radians.sin_cos();
	Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c).normalize_or_zero()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn degree1_reaches_near_end() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 100.0, 100.0);
		let graph = HysteresisGraph::degree1(
			bounds,
			42,
			Vec2::new(10.0, 10.0),
			Vec2::new(90.0, 90.0),
			&HysteresisConfig::default(),
		);
		assert!(!graph.nodes.is_empty());
		let last = *graph.nodes.last().ok_or_else(|| anyhow::anyhow!("empty path"))?;
		assert!(last.distance(Vec2::new(90.0, 90.0)) < 50.0);
		Ok(())
	}

	#[test]
	fn higher_degree_adds_nodes() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 200.0, 200.0);
		let d1 = HysteresisGraph::degree1(
			bounds,
			7,
			Vec2::new(20.0, 20.0),
			Vec2::new(180.0, 180.0),
			&HysteresisConfig::default(),
		);
		let d4 = HysteresisGraph::degree4(
			bounds,
			7,
			Vec2::new(20.0, 20.0),
			Vec2::new(180.0, 180.0),
			&HysteresisConfig::default(),
		);
		assert!(d4.nodes.len() >= d1.nodes.len());
		Ok(())
	}
}
