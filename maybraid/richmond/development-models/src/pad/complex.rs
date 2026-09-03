//! Indexed pad complex: member skirt nodes + sample-time flatten / ease.

use bevy::math::Vec2;
use durham_terrain_models::terrain::{ElevationModulation, TerrainSdf};
use procedural_common::Bounds2;

use super::node::{PadNode, PadStage};
use super::PadParams;

/// Bag of pad nodes blended by flatten-over-ease priority (HydroComplex analog).
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
		self
	}

	pub fn from_nodes(bounds: Bounds2, pads: Vec<PadNode>) -> Self {
		Self::new(bounds).with_pads(pads)
	}

	/// One rectangular flatten terrace for a yawed building in `bounds`.
	pub fn building_skirt(
		bounds: Bounds2,
		center: Vec2,
		building_half_extents: Vec2,
		yaw: f32,
		height: f32,
		params: PadParams,
	) -> Self {
		Self::from_nodes(
			bounds,
			vec![PadNode::rectangular_flatten(center, building_half_extents, yaw, height, params)],
		)
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
