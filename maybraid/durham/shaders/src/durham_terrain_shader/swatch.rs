//! Per-swatch GPU layout: `left` / `right` RGB endpoints and fold weight in `swatch_meta.x`.

use bevy::{prelude::*, render::render_resource::ShaderType};

/// Blend **`left.xyz` → `right.xyz`**; **`swatch_meta.x`** = fold-in weight (0–1). **`swatch_meta.yzw`** unused.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct DurhamSwatchUniform {
	pub left: Vec4,
	pub right: Vec4,
	pub swatch_meta: Vec4,
}

impl DurhamSwatchUniform {
	/// Linear ramp from `left_rgb` to `right_rgb` with shader fold weight `fold_weight`.
	pub fn transition(left_rgb: Vec3, right_rgb: Vec3, fold_weight: f32) -> Self {
		Self {
			left: left_rgb.extend(0.0),
			right: right_rgb.extend(0.0),
			swatch_meta: Vec4::new(fold_weight, 0.0, 0.0, 0.0),
		}
	}

	pub fn with_fold_weight(mut self, fold_weight: f32) -> Self {
		self.swatch_meta.x = fold_weight;
		self
	}

	pub fn with_left_rgb(mut self, rgb: Vec3) -> Self {
		self.left = rgb.extend(0.0);
		self
	}

	pub fn with_right_rgb(mut self, rgb: Vec3) -> Self {
		self.right = rgb.extend(0.0);
		self
	}
}
