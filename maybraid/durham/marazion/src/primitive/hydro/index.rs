//! Uniform-grid broadphase over conservative node AABBs.

use crate::primitive::node::HydrologyNode;
use bevy_math::Vec2;
use procedural_common::Bounds2;
use std::collections::HashMap;

/// Uniform-grid broadphase over conservative AABBs (not exact intersection bake).
#[derive(Debug, Clone)]
pub struct FootprintIndex {
	origin: Vec2,
	cell: f32,
	buckets: HashMap<(i32, i32), Vec<u16>>,
}

impl FootprintIndex {
	pub fn empty() -> Self {
		Self {
			origin: Vec2::ZERO,
			cell: 1.0,
			buckets: HashMap::new(),
		}
	}

	/// Broadphase over node hydraulic AABBs expanded by each node's index pad.
	pub fn build_nodes(bounds: Bounds2, nodes: &[HydrologyNode], cell: f32) -> Self {
		let cell = cell.max(1.0);
		let origin = bounds.min;
		let mut buckets: HashMap<(i32, i32), Vec<u16>> = HashMap::new();
		for (i, node) in nodes.iter().enumerate() {
			let id = i as u16;
			let (mn, mx) = node.primitive.aabb();
			let pad = node.index_pad();
			let i0 = ((mn.x - pad - origin.x) / cell).floor() as i32;
			let i1 = ((mx.x + pad - origin.x) / cell).floor() as i32;
			let j0 = ((mn.y - pad - origin.y) / cell).floor() as i32;
			let j1 = ((mx.y + pad - origin.y) / cell).floor() as i32;
			for ix in i0..=i1 {
				for iz in j0..=j1 {
					buckets.entry((ix, iz)).or_default().push(id);
				}
			}
		}
		Self {
			origin,
			cell,
			buckets,
		}
	}

	pub fn candidates(&self, p: Vec2) -> &[u16] {
		let ix = ((p.x - self.origin.x) / self.cell).floor() as i32;
		let iz = ((p.y - self.origin.y) / self.cell).floor() as i32;
		self.buckets
			.get(&(ix, iz))
			.map(|v| v.as_slice())
			.unwrap_or(&[])
	}
}
