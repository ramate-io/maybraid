//! Closed rectangular strip whose stations sit on a sampled ground height.
//!
//! Each node keeps roll `0` (top toward `+Y`). The panel rises from
//! `min(terrain, plaza)` high enough to clear the plaza, so a slope beside a
//! flat courtyard still seals the edge.

use bevy_math::Vec2;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::{RectangularStrip, RectangularStripNode, DEFAULT_PANEL_THICKNESS};

/// Terrain-following perimeter built from [`RectangularStrip`] bays.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainPerimeterWall {
	strip: RectangularStrip,
}

impl TerrainPerimeterWall {
	/// Stations around an axis-aligned XZ rectangle, excluding the repeated close.
	pub fn sample_rectangle(min: Vec2, max: Vec2, step: f32) -> Vec<Vec2> {
		let (min, max) = (min.min(max), min.max(max));
		let mut points = Vec::new();
		push_edge(&mut points, Vec2::new(min.x, min.y), Vec2::new(max.x, min.y), step);
		push_edge(&mut points, Vec2::new(max.x, min.y), Vec2::new(max.x, max.y), step);
		push_edge(&mut points, Vec2::new(max.x, max.y), Vec2::new(min.x, max.y), step);
		push_edge(&mut points, Vec2::new(min.x, max.y), Vec2::new(min.x, min.y), step);
		points
	}

	/// `terrain_y[i]` is the ground height at `samples[i]`. The strip repeats the
	/// first station so the last bay closes the rectangle.
	pub fn from_samples(samples: &[Vec2], terrain_y: &[f32], plaza_y: f32, clearance: f32) -> Self {
		let clearance = clearance.max(0.5);
		let n = samples.len().min(terrain_y.len());
		let mut nodes = Vec::with_capacity(n + 1);
		for i in 0..n {
			nodes.push(station(samples[i], terrain_y[i], plaza_y, clearance));
		}
		if let Some(first) = nodes.first().copied() {
			nodes.push(first);
		}
		Self { strip: RectangularStrip::from_nodes(PanelStyle::RoughStonework, nodes) }
	}

	pub fn strip(&self) -> &RectangularStrip {
		&self.strip
	}
}

fn station(xz: Vec2, terrain_y: f32, plaza_y: f32, clearance: f32) -> RectangularStripNode {
	let base = terrain_y.min(plaza_y);
	let height = (plaza_y + clearance - base).max(clearance);
	RectangularStripNode::new(
		bevy_math::Vec3::new(xz.x, base, xz.y),
		height,
		DEFAULT_PANEL_THICKNESS,
		0.0,
	)
}

fn push_edge(out: &mut Vec<Vec2>, from: Vec2, to: Vec2, step: f32) {
	let delta = to - from;
	let len = delta.length();
	if len <= 1e-4 {
		return;
	}
	let n = (len / step.max(0.5)).ceil().max(1.0) as usize;
	for i in 0..n {
		out.push(from + delta * (i as f32 / n as f32));
	}
}

impl BuildingComponents for TerrainPerimeterWall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.strip.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.strip.joint_nodes_for_level(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn closed_strip_clears_the_plaza_on_a_slope() {
		let samples = TerrainPerimeterWall::sample_rectangle(
			Vec2::new(-10.0, -8.0),
			Vec2::new(10.0, 8.0),
			8.0,
		);
		assert!(samples.len() >= 4);
		let terrain: Vec<f32> = samples.iter().map(|p| p.x * 0.25).collect();
		let wall = TerrainPerimeterWall::from_samples(&samples, &terrain, 5.0, 4.0);
		let nodes = wall.strip().nodes();
		assert_eq!(nodes.len(), samples.len() + 1);
		assert_eq!(nodes.first().map(|n| n.position), nodes.last().map(|n| n.position));
		for (node, y) in nodes.iter().zip(terrain.iter().chain(terrain.first())) {
			let base = y.min(5.0);
			assert!((node.position.y - base).abs() < 1e-4);
			assert!((node.position.y + node.height - 9.0).abs() < 1e-3);
			assert_eq!(node.roll, 0.0);
		}
		assert!(!wall.strip().bays().is_empty());
	}
}
