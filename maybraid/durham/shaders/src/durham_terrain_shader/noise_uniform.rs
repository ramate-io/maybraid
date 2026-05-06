//! Full `terrain_noise` uniform: regional blend driver + four bands.

use bevy::{prelude::*, render::render_resource::ShaderType};

use super::band::DurhamTerrainBandUniform;
use super::palettes::{macro_region_palette, micro_region_palette};

/// Equal contribution from each frequency band when combining (shader uses this as `band_scale.z`).
pub const EVEN_BAND_BLEND_WEIGHT: f32 = 0.25;

/// **`regional_blend`**: **`x`** = `t_warp` FBM frequency, **`y`** = amplitude; **`zw`** unused.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct DurhamTerrainNoiseUniform {
	pub regional_blend: Vec4,
	pub bands: [DurhamTerrainBandUniform; 4],
}

impl DurhamTerrainNoiseUniform {
	pub fn with_regional_blend_frequency(mut self, frequency: f32) -> Self {
		self.regional_blend.x = frequency;
		self
	}

	pub fn with_regional_blend_amplitude(mut self, amplitude: f32) -> Self {
		self.regional_blend.y = amplitude;
		self
	}

	pub fn with_band(mut self, index: usize, band: DurhamTerrainBandUniform) -> Self {
		if index < 4 {
			self.bands[index] = band;
		}
		self
	}
}

impl Default for DurhamTerrainNoiseUniform {
	fn default() -> Self {
		let macro_sw = macro_region_palette();
		let micro_sw = micro_region_palette();
		Self {
			regional_blend: Vec4::new(0.00015, 0.5, 0.0, 0.0),
			bands: [
				DurhamTerrainBandUniform::new(42.0, 0.000001, 0.5, 0.30, macro_sw),
				DurhamTerrainBandUniform::new(42.0, 0.0001, 0.5, 0.50, macro_sw),
				DurhamTerrainBandUniform::new(42.0, 0.01, 0.5, 0.10, micro_sw),
				DurhamTerrainBandUniform::new(42.0, 0.1, 0.4, 0.10, micro_sw),
			],
		}
	}
}
