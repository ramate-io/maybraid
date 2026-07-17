//! Softmask affine height operator: `a * elevation + b` inside a region.

use crate::region::{Region2D, RegionNoise};
use bevy_math::Vec2;

/// Softmask affine: `a * elevation + b` inside a region.
#[derive(Debug, Clone)]
pub struct RegionAffineModulation {
	pub region: Region2D,
	pub inner_scale: f32,
	pub inner_offset: f32,
	pub inner_radius: f32,
	pub outer_radius: f32,
	pub noise: Option<RegionNoise>,
}

impl RegionAffineModulation {
	pub fn new(
		region: Region2D,
		inner_scale: f32,
		inner_offset: f32,
		inner_radius: f32,
		outer_radius: f32,
	) -> Self {
		Self {
			region,
			inner_scale,
			inner_offset,
			inner_radius,
			outer_radius: outer_radius.max(inner_radius + 0.001),
			noise: None,
		}
	}

	pub fn with_noise(mut self, noise: RegionNoise) -> Self {
		self.noise = Some(noise);
		self
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let w = self.region.softmask_weight(
			Vec2::new(x, z),
			self.inner_radius,
			self.outer_radius,
			self.noise.as_ref(),
		);
		let a = self.inner_scale + (1.0 - self.inner_scale) * w;
		let b = self.inner_offset * (1.0 - w);
		a * elevation + b
	}
}
