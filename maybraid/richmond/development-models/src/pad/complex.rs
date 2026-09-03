//! Indexed pad complex: member skirt nodes + sample-time flatten / ease.

use bevy::math::Vec2;
use durham_terrain_models::terrain::{ElevationModulation, TerrainSdf};
use procedural_common::Bounds2;

use super::node::{PadNode, PadStage};
use super::{nodes_from_graded_polyline, PadParams};

/// Bag of pad nodes blended by occupancy softmax, with flatten terraces exact.
///
/// [`Self::bounds`] is the union of each node's yawed support AABB (flatten +
/// ease), not the development cell. Early-out is a conservative OBB AABB;
/// classification still uses the yawed footprint SDF.
#[derive(Debug, Clone)]
pub struct PadComplex {
	pub bounds: Bounds2,
	pub pads: Vec<PadNode>,
}

impl PadComplex {
	pub fn new(bounds: Bounds2) -> Self {
		Self { bounds, pads: Vec::new() }
	}

	pub fn with_pads(mut self, pads: Vec<PadNode>) -> Self {
		self.pads = pads;
		self.bounds = union_pad_bounds(&self.pads);
		self
	}

	pub fn from_nodes(pads: Vec<PadNode>) -> Self {
		let bounds = union_pad_bounds(&pads);
		Self { bounds, pads }
	}

	/// Flatten / grade / ease must blend in one pass. Sequential complexes let a
	/// later ease skirt undo an earlier terrace and smear overlapping path
	/// skirts into a general lift.
	pub fn union_all(complexes: impl IntoIterator<Item = Self>) -> Self {
		Self::from_nodes(complexes.into_iter().flat_map(|c| c.pads).collect())
	}

	/// One rectangular flatten terrace for a yawed building plan.
	pub fn building_skirt(
		center: Vec2,
		building_half_extents: Vec2,
		yaw: f32,
		height: f32,
		params: PadParams,
	) -> Self {
		Self::from_nodes(vec![PadNode::rectangular_flatten(
			center,
			building_half_extents,
			yaw,
			height,
			params,
		)])
	}

	/// One graded reach node per polyline segment, analog of hydro `nodes_from_polyline`.
	pub fn graded_polyline(
		path: &[Vec2],
		levels: &[f32],
		half_width: f32,
		params: PadParams,
	) -> Self {
		Self::from_nodes(nodes_from_graded_polyline(path, levels, half_width, params))
	}

	pub fn is_empty(&self) -> bool {
		self.pads.is_empty()
	}

	pub fn nodes_intersecting(&self, p: Vec2) -> Vec<&PadNode> {
		self.pads.iter().filter(|node| node.contains_index_point(p)).collect()
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		if !self.bounds.contains(p) {
			return elevation;
		}
		let nodes = self.nodes_intersecting(p);
		PadNode::elevation_blend(&nodes, elevation, p)
	}

	pub fn classification_at(&self, x: f32, z: f32) -> Option<PadStage> {
		let p = Vec2::new(x, z);
		let mut best: Option<(&PadNode, f32)> = None;
		for node in self.nodes_intersecting(p) {
			let phi = node.phi(p);
			if best.map(|(_, d)| phi < d).unwrap_or(true) {
				best = Some((node, phi));
			}
		}
		best.and_then(|(n, _)| n.classification(p))
	}
}

impl ElevationModulation for PadComplex {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		PadComplex::modify_elevation(self, elevation, x, z)
	}
}

fn union_pad_bounds(pads: &[PadNode]) -> Bounds2 {
	let mut min = Vec2::splat(f32::INFINITY);
	let mut max = Vec2::splat(f32::NEG_INFINITY);
	for node in pads {
		let b = node.correction_index_bounds();
		min = min.min(b.min);
		max = max.max(b.max);
	}
	if !min.is_finite() {
		return Bounds2::from_xz(0.0, 0.0, 0.0, 0.0);
	}
	Bounds2 { min, max }
}
