//! [`VegetationGenerationPlugin`]: forest, grove, and bump-out recipes.
//!
//! Vegetation generation reads no terrain, so it is untyped. Terrain enters at
//! presentation through `VegetationPresentationPlugin<Mode, G>`.

mod config;
mod generation;
mod stream;

pub use config::{ForestStreamSpec, VegetationLayerConfig};
pub use generation::{
	VegetationGenerationCore, VegetationGenerationPlugin, VegetationGenerationSystems,
	VegetationModeConfig,
};
pub use stream::{
	clear_vegetation_stream, install_vegetation_stream, stream_canopy_bump_outs, stream_forest,
	stream_radii_m, stream_vegetation, VegetationStreamKey, DEFAULT_FOREST_NOISE,
	DEFAULT_FOREST_STREAM_RADIUS,
};
