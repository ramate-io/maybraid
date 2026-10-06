//! [`LanguageModel`]: what [`crate::Language`] and language presentation read.

use bevy::app::App;
use terrain_layer_model::TerrainModel;

/// Named language / place-naming over a vegetated world.
///
/// Only what [`crate::Language<Self>`] and language presentation read. No method
/// exists for a higher layer. Durham, Chico, and Richmond do not grow a naming
/// API for this.
pub trait LanguageModel: Send + Sync + 'static {
	/// Vegetated world under this layer. Heights stay on that world.
	type World: TerrainModel;
	/// One large-tile language cell.
	type Cell: Send + Sync + 'static;

	fn require_generation(app: &App);
}
