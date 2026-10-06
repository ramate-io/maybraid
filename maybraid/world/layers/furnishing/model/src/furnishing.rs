//! [`FurnishingModel`]: what [`crate::Furnishing`] and furnishing presentation read.

use bevy::app::App;
use terrain_layer_model::TerrainModel;

/// Named furnishing over a ground model.
///
/// Only what [`crate::Furnishing<Self>`] and furnishing presentation read: a
/// cell of world-space slots. No method exists for a higher layer.
pub trait FurnishingModel: Send + Sync + 'static {
	type Ground: TerrainModel;
	/// One cell of world-space furniture slots.
	type Cell: Send + Sync + 'static;

	fn require_generation(app: &App);
}
