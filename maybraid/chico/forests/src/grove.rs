//! Generated 100 m grove on one forest layer.

use std::sync::OnceLock;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Vec3;
use chico_groves::{GroveExtent, GroveWorldSample};
use lod::gen::Id;

use crate::{ForestGroveRecipe, ForestGroveTile, ForestLayer};

/// One layer on a 100 m tile: blend recipes, plus grown tiles after present
/// (or a test) calls [`Self::ensure_grown`]. Generate stores recipes only.
pub struct ChicoGrove {
	pub extent: GroveExtent,
	pub layer: ForestLayer,
	pub recipes: Vec<ForestGroveRecipe>,
	grown: OnceLock<Vec<ForestGroveTile>>,
}

impl Clone for ChicoGrove {
	fn clone(&self) -> Self {
		let grown = OnceLock::new();
		if let Some(tiles) = self.grown.get() {
			let _ = grown.set(tiles.clone());
		}
		Self { extent: self.extent, layer: self.layer, recipes: self.recipes.clone(), grown }
	}
}

impl ChicoGrove {
	pub fn selected(
		extent: GroveExtent,
		layer: ForestLayer,
		recipes: Vec<ForestGroveRecipe>,
	) -> Self {
		Self { extent, layer, recipes, grown: OnceLock::new() }
	}

	pub fn id(&self) -> Id {
		grove_id(self.extent, self.layer)
	}

	pub fn aabb(&self) -> Aabb3d {
		grove_aabb(self.extent, self.layer)
	}

	/// Grown tiles if [`Self::ensure_grown`] (or [`Self::grow`]) has run.
	pub fn grown_tiles(&self) -> Option<&[ForestGroveTile]> {
		self.grown.get().map(Vec::as_slice)
	}

	/// Grow recipes into storage once. Present handle grows on one tick and
	/// spawns on the next so a dense tile does not own both costs.
	pub fn ensure_grown(&self, world: &impl GroveWorldSample) -> &[ForestGroveTile] {
		self.grown.get_or_init(|| self.grow(world))
	}

	/// Tiles to spawn this present slot. `None` means this call grew and the
	/// caller should wait for the next slot.
	pub fn tiles_ready_to_present(
		&self,
		world: &impl GroveWorldSample,
	) -> Option<&[ForestGroveTile]> {
		if self.grown.get().is_some() {
			return self.grown_tiles();
		}
		self.ensure_grown(world);
		None
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
	fn ensure_grown_is_once_and_leaves_recipes() -> Result<()> {
		use crate::index::forest_world_sample;
		use crate::{ForestGroveKind, ForestGroveRecipe};

		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let grove = ChicoGrove::selected(
			extent,
			ForestLayer::UpperCanopy,
			vec![ForestGroveRecipe::uniform(ForestGroveKind::Orchard, extent)],
		);
		assert!(grove.grown_tiles().is_none());
		let first = grove.ensure_grown(&forest_world_sample()).len();
		assert!(first > 0);
		assert_eq!(grove.ensure_grown(&forest_world_sample()).len(), first);
		assert!(!grove.recipes.is_empty());
		Ok(())
	}

	#[test]
	fn tiles_ready_to_present_grows_then_returns_tiles() -> Result<()> {
		use crate::index::forest_world_sample;
		use crate::{ForestGroveKind, ForestGroveRecipe};

		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let grove = ChicoGrove::selected(
			extent,
			ForestLayer::UpperCanopy,
			vec![ForestGroveRecipe::uniform(ForestGroveKind::Orchard, extent)],
		);
		assert!(grove.tiles_ready_to_present(&forest_world_sample()).is_none());
		assert!(grove.grown_tiles().is_some());
		let tiles = grove
			.tiles_ready_to_present(&forest_world_sample())
			.ok_or_else(|| anyhow::anyhow!("ready"))?;
		assert!(!tiles.is_empty());
		Ok(())
	}

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
