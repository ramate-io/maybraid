//! Durham terrain shaders: reusable Bevy materials and embedded WGSL for Maybraid terrain work.

mod durham_terrain_shader;

pub use durham_terrain_shader::{
	macro_region_palette, micro_region_palette, DurhamSwatchUniform,
	DurhamTerrainBandUniform, DurhamTerrainNoiseUniform, DurhamTerrainShader,
	DurhamTerrainShaderPlugin, EVEN_BAND_BLEND_WEIGHT, EVEN_SWATCH_FOLD_WEIGHT,
};
