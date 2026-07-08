//! Half-Lambert wrap, smoothstep, and value-band controls.

use bevy::render::render_resource::ShaderType;

/// Lighting and tonal shaping for [`super::WatercolorShader`].
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct WatercolorLightingUniform {
	/// Number of soft value bands (e.g. `4.0`).
	pub band_count: f32,
	/// Blend toward quantized bands (`0.0` = smooth, `1.0` = hard steps).
	pub band_mix: f32,
	/// Lower edge of the lighting smoothstep.
	pub light_smooth_min: f32,
	/// Upper edge of the lighting smoothstep.
	pub light_smooth_max: f32,
	/// Half-Lambert scale on `dot(normal, light_dir)`.
	pub diffuse_scale: f32,
	/// Half-Lambert bias added after scaling.
	pub diffuse_bias: f32,
	/// Lighting level when no directional lights are present.
	pub fallback_light: f32,
}

impl WatercolorLightingUniform {
	pub fn new(
		band_count: f32,
		band_mix: f32,
		light_smooth_min: f32,
		light_smooth_max: f32,
		diffuse_scale: f32,
		diffuse_bias: f32,
		fallback_light: f32,
	) -> Self {
		Self {
			band_count,
			band_mix,
			light_smooth_min,
			light_smooth_max,
			diffuse_scale,
			diffuse_bias,
			fallback_light,
		}
	}

	#[inline]
	pub fn with_band_count(mut self, band_count: f32) -> Self {
		self.band_count = band_count;
		self
	}

	#[inline]
	pub fn with_band_mix(mut self, band_mix: f32) -> Self {
		self.band_mix = band_mix;
		self
	}

	#[inline]
	pub fn with_light_smooth_min(mut self, light_smooth_min: f32) -> Self {
		self.light_smooth_min = light_smooth_min;
		self
	}

	#[inline]
	pub fn with_light_smooth_max(mut self, light_smooth_max: f32) -> Self {
		self.light_smooth_max = light_smooth_max;
		self
	}

	#[inline]
	pub fn with_diffuse_wrap(mut self, scale: f32, bias: f32) -> Self {
		self.diffuse_scale = scale;
		self.diffuse_bias = bias;
		self
	}

	#[inline]
	pub fn with_fallback_light(mut self, fallback_light: f32) -> Self {
		self.fallback_light = fallback_light;
		self
	}
}

impl Default for WatercolorLightingUniform {
	fn default() -> Self {
		Self {
			band_count: 16.0,
			band_mix: 0.35,
			light_smooth_min: 0.3,
			light_smooth_max: 0.78,
			diffuse_scale: 0.55,
			diffuse_bias: 0.25,
			fallback_light: 0.65,
		}
	}
}
