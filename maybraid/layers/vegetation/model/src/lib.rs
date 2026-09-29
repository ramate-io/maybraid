//! [`VegetationGenerationPlugin`]: forest, grove, and bump-out recipes.
//!
//! Vegetation generation reads no terrain, so it is untyped. Terrain enters at
//! presentation through `VegetationPresentationPlugin<G>`.

use bevy::app::{App, Plugin};

/// Forest / grove / bump-out selection into `ForestIndex`. No grow, no hosts.
#[derive(Default)]
pub struct VegetationGenerationPlugin;

impl Plugin for VegetationGenerationPlugin {
	fn build(&self, _app: &mut App) {
		todo!("VegetationGenerationPlugin: fill from maybraid/layers/vegetation/model/src/lib_sketch.rs")
	}
}
