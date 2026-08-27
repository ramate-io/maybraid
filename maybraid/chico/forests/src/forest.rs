//! Chico forest model: Hopscotch a cell, then grow the selected grove tiles.
//!
//! This is a generation result, not a `LodScene` host. Playgrounds spawn the
//! concrete groves underneath.

use chico_groves::GroveWorldSample;
use procedural_common::NoiseParams;

use crate::{
	assemble, select_cell, AssembledForest, ForestExtent, ForestGroveTile, LayeringKind,
	NeighborLayers,
};

/// One assembled forest cell (extent + grown grove tiles).
#[derive(Clone)]
pub struct ChicoForest {
	pub extent: ForestExtent,
	pub assembled: AssembledForest,
}

impl ChicoForest {
	/// Hopscotch + per-layer Bucket Throw, then grow default groves on the cell grid.
	pub fn assemble_on(
		extent: ForestExtent,
		noise: NoiseParams,
		world: &impl GroveWorldSample,
	) -> Self {
		let layers = select_cell(extent, noise);
		let neighbors = neighbor_layers(extent, noise);
		Self { extent, assembled: assemble(extent, layers, neighbors, world) }
	}

	/// Pin a well-known layering and grow its typical (highest-weight) groves.
	pub fn assemble_layering(
		extent: ForestExtent,
		layering: LayeringKind,
		world: &impl GroveWorldSample,
	) -> Self {
		let layers = layering.layering().typical_layers();
		// Pinned review cells share one layering; no kind change on faces.
		Self { extent, assembled: assemble(extent, layers, NeighborLayers::none(), world) }
	}

	pub fn tiles(&self) -> impl Iterator<Item = &ForestGroveTile> {
		self.assembled.tiles()
	}
}

/// Hopscotch the four cardinal neighbors (selection only).
pub fn neighbor_layers(extent: ForestExtent, noise: NoiseParams) -> NeighborLayers {
	let (ix, iz) = ForestExtent::cell_index_containing(extent.center());
	NeighborLayers {
		north: Some(select_cell(ForestExtent::from_cell_index(ix, iz + 1), noise)),
		east: Some(select_cell(ForestExtent::from_cell_index(ix + 1, iz), noise)),
		south: Some(select_cell(ForestExtent::from_cell_index(ix, iz - 1), noise)),
		west: Some(select_cell(ForestExtent::from_cell_index(ix - 1, iz), noise)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;
	use chico_groves::FlatTerrainSample;

	#[test]
	fn assemble_on_one_tile_cell_grows_selected_layers() -> Result<()> {
		let extent = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let forest =
			ChicoForest::assemble_on(extent, NoiseParams::default(), &FlatTerrainSample::default());
		assert_eq!(forest.extent, extent);
		let tile_count = forest.tiles().count();
		let selected = [
			forest.assembled.layers.tufts,
			forest.assembled.layers.understory,
			forest.assembled.layers.lower_canopy,
			forest.assembled.layers.upper_canopy,
		]
		.into_iter()
		.filter(Option::is_some)
		.count();
		assert!(
			tile_count >= selected,
			"each selected layer grows at least one host (blend can add more): tiles={tile_count} layers={selected}"
		);
		Ok(())
	}

	#[test]
	fn assemble_layering_uses_typical_lush_jungle_groves() -> Result<()> {
		let extent = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let forest = ChicoForest::assemble_layering(
			extent,
			LayeringKind::LushJungle,
			&FlatTerrainSample::default(),
		);
		assert_eq!(forest.assembled.layers.layering, LayeringKind::LushJungle);
		assert!(forest.tiles().next().is_some());
		Ok(())
	}
}
