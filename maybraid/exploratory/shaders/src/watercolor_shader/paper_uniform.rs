//! Large soft paper grain modulating brightness.

use bevy::render::render_resource::ShaderType;

/// Paper / noise variation for [`super::WatercolorShader`].
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct WatercolorPaperUniform {
	/// World-space noise frequency.
	pub noise_scale: f32,
	/// Multiplier on noise before adding to brightness base.
	pub noise_strength: f32,
	/// Base brightness before paper variation.
	pub brightness_base: f32,
	/// Seed offset for the paper noise hash.
	pub seed: f32,
}

impl WatercolorPaperUniform {
	pub fn new(noise_scale: f32, noise_strength: f32, brightness_base: f32, seed: f32) -> Self {
		Self { noise_scale, noise_strength, brightness_base, seed }
	}

	#[inline]
	pub fn with_noise_scale(mut self, noise_scale: f32) -> Self {
		self.noise_scale = noise_scale;
		self
	}

	#[inline]
	pub fn with_noise_strength(mut self, noise_strength: f32) -> Self {
		self.noise_strength = noise_strength;
		self
	}

	#[inline]
	pub fn with_brightness_base(mut self, brightness_base: f32) -> Self {
		self.brightness_base = brightness_base;
		self
	}

	#[inline]
	pub fn with_seed(mut self, seed: f32) -> Self {
		self.seed = seed;
		self
	}
}

impl Default for WatercolorPaperUniform {
	fn default() -> Self {
		Self::new(3.0, 0.15, 0.84, 42.0)
	}
}
