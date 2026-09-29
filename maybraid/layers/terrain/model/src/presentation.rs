//! Model-owned presentation install.
//!
//! Symmetric with [`TerrainGeneration`](crate::TerrainGeneration): a presentation
//! plugin calls this hook instead of drawing cells itself. Unifying raw and
//! padded presenters behind [`TerrainCell`](crate::TerrainCell) is a follow-up.

use bevy::app::App;

use crate::model::TerrainModel;

/// A model that installs its own cell presentation.
pub trait TerrainPresentation: TerrainModel {
	/// Register presenter state and present systems. No generation.
	fn install_presentation(app: &mut App);
}
