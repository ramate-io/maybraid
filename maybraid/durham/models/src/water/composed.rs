//! Water volume SDF for meshing stamp-owned pocket fills.

use crate::terrain::sdf::TerrainSdf;
use crate::water::water_distance as fill_water_distance;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use marazion_watersheds::WaterFill;
use render_item::mesh::{IdentifiedMesh, MeshId};
use render_item::NormalizeChunk;
use sdf::{Sdf, Sign, SignBoundary, SignUniformIntervals};

/// Softmask weight above which a column is treated as dry for Y-interval skips.
const SOFTMASK_DRY: f32 = 0.999;

/// Meshable water volume: union of stamp fills against a composed heightfield.
#[derive(Clone, Debug)]
pub struct ComposedWater {
	pub terrain: TerrainSdf,
	pub fills: Vec<WaterFill>,
}

impl ComposedWater {
	pub fn new(terrain: TerrainSdf, fills: Vec<WaterFill>) -> Self {
		Self { terrain, fills }
	}
}

impl Sdf for ComposedWater {
	fn distance(&self, p: Vec3) -> f32 {
		let h = self.terrain.height_at_with_all_modulations(p.x, p.z);
		fill_water_distance(&self.fills, p, h)
	}

	fn sign_uniform_on_y(&self, x: f32, z: f32) -> SignUniformIntervals {
		let h = self.terrain.height_at_with_all_modulations(x, z);
		let p_xz = Vec2::new(x, z);

		let mut wet_bottom = f32::INFINITY;
		let mut wet_top = f32::NEG_INFINITY;
		for fill in &self.fills {
			let w = fill.region.softmask_weight(
				p_xz,
				fill.inner_radius,
				fill.outer_radius,
				fill.noise.as_ref(),
			);
			if w >= SOFTMASK_DRY {
				continue;
			}
			if fill.water_level > h {
				wet_bottom = wet_bottom.min(h);
				wet_top = wet_top.max(fill.water_level);
			}
		}

		let mut intervals = SignUniformIntervals::default();
		intervals.insert_boundary(SignBoundary {
			min: f32::NEG_INFINITY,
			sign: Sign::Positive,
		});
		if wet_bottom.is_finite() && wet_top > wet_bottom {
			intervals.insert_boundary(SignBoundary {
				min: wet_bottom,
				sign: Sign::Negative,
			});
			intervals.insert_boundary(SignBoundary {
				min: wet_top,
				sign: Sign::Positive,
			});
		}
		intervals
	}
}

impl NormalizeChunk for ComposedWater {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		cascade_chunk.clone()
	}
}

impl IdentifiedMesh for ComposedWater {
	fn id(&self) -> MeshId {
		MeshId::new(format!("{:?}", self))
	}
}
