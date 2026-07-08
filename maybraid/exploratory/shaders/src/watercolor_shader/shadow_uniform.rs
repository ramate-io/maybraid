//! Cool shadow hue tint multiplied into the base color in shade.

use bevy::{prelude::*, render::render_resource::ShaderType};

/// Shadow color bleeding for [`super::WatercolorShader`].
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct WatercolorShadowUniform {
	pub tint_r: f32,
	pub tint_g: f32,
	pub tint_b: f32,
	pub _pad: f32,
}

impl WatercolorShadowUniform {
	pub fn new(tint_r: f32, tint_g: f32, tint_b: f32) -> Self {
		Self { tint_r, tint_g, tint_b, _pad: 0.0 }
	}

	pub fn from_rgb(rgb: Vec3) -> Self {
		Self::new(rgb.x, rgb.y, rgb.z)
	}

	pub fn from_color(color: Color) -> Self {
		let c = color.to_srgba();
		Self::new(c.red, c.green, c.blue)
	}

	#[inline]
	pub fn tint(&self) -> Vec3 {
		Vec3::new(self.tint_r, self.tint_g, self.tint_b)
	}

	#[inline]
	pub fn with_tint(mut self, tint: Vec3) -> Self {
		self.tint_r = tint.x;
		self.tint_g = tint.y;
		self.tint_b = tint.z;
		self
	}

	#[inline]
	pub fn with_color(mut self, color: Color) -> Self {
		let c = color.to_srgba();
		self.tint_r = c.red;
		self.tint_g = c.green;
		self.tint_b = c.blue;
		self
	}
}

impl Default for WatercolorShadowUniform {
	fn default() -> Self {
		Self::new(0.42, 0.52, 0.68)
	}
}
