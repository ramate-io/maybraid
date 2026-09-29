//! [`PadComposable`]: inner cells that development pads can be composed into.

use durham_terrain_models::Terrain;
use richmond_development_models::{PadComplex, TerrainWithPads};
use terrain_layer_model::TerrainCell;

/// An inner model's cell that accepts pad elevation ops.
pub trait PadComposable: TerrainCell {
	/// The composed cell stored for `Urbanization<M>`.
	type Padded: TerrainCell;

	fn compose_pads(&self, pads: &PadComplex) -> Self::Padded;
}

impl PadComposable for Terrain {
	type Padded = TerrainWithPads;

	fn compose_pads(&self, pads: &PadComplex) -> TerrainWithPads {
		TerrainWithPads::compose(self, std::iter::once(pads))
	}
}
