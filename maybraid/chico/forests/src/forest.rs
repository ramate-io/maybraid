//! Chico forest model: Hopscotch a cell, then grow the selected grove tiles.
//!
//! This is a generation result, not a `LodScene` host. Playgrounds spawn the
//! concrete groves underneath.

use chico_groves::GroveWorldSample;
use procedural_common::NoiseParams;

use crate::{assemble, select_cell, AssembledForest, ForestExtent, ForestGroveTile, LayeringKind};

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
		Self { extent, assembled: assemble(extent, layers, world) }
	}

	/// Pin a well-known layering and grow its typical (highest-weight) groves.
	pub fn assemble_layering(
		extent: ForestExtent,
		layering: LayeringKind,
		world: &impl GroveWorldSample,
	) -> Self {
		let layers = layering.layering().typical_layers();
		Self { extent, assembled: assemble(extent, layers, world) }
	}

	pub fn tiles(&self) -> impl Iterator<Item = &ForestGroveTile> {
		self.assembled.tiles()
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
		assert_eq!(tile_count, selected);
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
