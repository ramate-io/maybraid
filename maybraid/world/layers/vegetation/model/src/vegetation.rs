//! [`VegetationModel`]: what [`crate::Vegetation`] and vegetation presentation read.

use bevy::app::App;
use terrain_layer_model::TerrainModel;

/// Named vegetation over a ground model.
///
/// Only what [`crate::Vegetation<Self>`]'s [`TerrainModel`] impl and vegetation
/// presentation read. No method exists for a higher layer.
pub trait VegetationModel: Send + Sync + 'static {
	type Ground: TerrainModel;

	fn require_generation(app: &App);
}
