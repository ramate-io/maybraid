//! Durham terrain shaders: reusable Bevy materials and embedded WGSL for Maybraid terrain work.

mod terrain_shader;
mod refraction_water;

pub use terrain_shader::{
	TerrainSwatchUniform, TerrainBandUniform, TerrainNoiseUniform, TerrainShader,
	TerrainShaderPlugin, EVEN_BAND_BLEND_WEIGHT, EVEN_SWATCH_FOLD_WEIGHT,
};
pub use refraction_water::{RefractionWater, RefractionWaterPlugin};
