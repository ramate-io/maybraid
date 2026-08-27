//! Generated 100 m grove on one forest layer.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Vec3;
use chico_groves::{GroveExtent, GroveWorldSample};
use lod::gen::Id;

use crate::{ForestGroveRecipe, ForestGroveTile, ForestLayer};

/// One layer on a 100 m tile: blend recipes, not grown plants.
#[derive(Clone)]
pub struct ChicoGrove {
	pub extent: GroveExtent,
	pub layer: ForestLayer,
	pub recipes: Vec<ForestGroveRecipe>,
}

impl ChicoGrove {
	pub fn id(&self) -> Id {
		grove_id(self.extent, self.layer)
	}

	pub fn aabb(&self) -> Aabb3d {
		grove_aabb(self.extent, self.layer)
	}

	pub fn grow(&self, world: &impl GroveWorldSample) -> Vec<ForestGroveTile> {
		self.recipes.iter().map(|recipe| recipe.grow(world)).collect()
	}
}

/// Origin-cell id for `(tile, layer)`. Layer is encoded in Y so stacked layers
/// on the same footprint stay distinct and camera-distance sort still uses XZ.
pub fn grove_id(extent: GroveExtent, layer: ForestLayer) -> Id {
	Id::from_cell(grove_aabb(extent, layer))
}

pub fn grove_from_id(id: Id) -> Option<(GroveExtent, ForestLayer)> {
	let bounds = id.origin_cell_bounds()?;
	let layer = ForestLayer::from_id_y(bounds.min.y)?;
	let extent = GroveExtent::new(
		Vec3::new(bounds.min.x, 0.0, bounds.min.z),
		Vec3::new(bounds.max.x, 1.0, bounds.max.z),
	);
	Some((extent, layer))
}

fn grove_aabb(extent: GroveExtent, layer: ForestLayer) -> Aabb3d {
	let y = layer.id_y();
	Aabb3d::from_min_max(
		Vec3::new(extent.min().x, y, extent.min().z),
		Vec3::new(extent.max().x, y + 1.0, extent.max().z),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use chico_groves::GroveExtent;

	#[test]
	fn grove_id_round_trips_layer() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		for layer in ForestLayer::ALL {
			let (decoded, got) =
				grove_from_id(grove_id(extent, layer)).ok_or_else(|| anyhow::anyhow!("id"))?;
			assert_eq!(got, layer);
			assert!((decoded.min().x - extent.min().x).abs() < 1e-4);
		}
		Ok(())
	}
}
