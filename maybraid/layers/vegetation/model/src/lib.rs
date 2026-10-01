//! [`VegetationGenerationPlugin`]: forest, grove, and bump-out recipes.
//!
//! Vegetation generation reads no terrain, so it is untyped. Terrain enters at
//! presentation through `VegetationPresentationPlugin<Mode, G>`.

mod config;
mod generation;
mod stream;

pub use config::{ForestStreamSpec, VegetationLayerConfig};
pub use generation::{VegetationGenerationPlugin, VegetationGenerationSystems};
pub use stream::{
	stream_canopy_bump_outs, stream_forest, stream_radii_m, DEFAULT_FOREST_NOISE,
	DEFAULT_FOREST_STREAM_RADIUS,
};
