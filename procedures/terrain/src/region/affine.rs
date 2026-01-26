use crate::region::{Region2D, RegionNoise};
use crate::{ElevationModulation, TerrainSdf};
use bevy::prelude::*;

/// A unified modulation: applies both scaling (`a`) and offset (`b`) inside a smooth region.
/// Outside region → a = 1, b = 0
/// Inside region → a = inner_scale, b = extrusion_height
#[derive(Debug, Clone)]
pub struct RegionAffineModulation {
	pub region: Region2D,
	pub inner_scale: f32,
	pub inner_offset: f32,
	pub inner_radius: f32,
	pub outer_radius: f32,
	/// Optional noise for perturbing the region boundary
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

	/// Add noise perturbation to the region boundary
	pub fn with_noise(mut self, noise: RegionNoise) -> Self {
		self.noise = Some(noise);
		self
	}

	#[inline(always)]
	fn smoothstep(t: f32) -> f32 {
		let t = t.clamp(0.0, 1.0);
		t * t * (3.0 - 2.0 * t)
	}

	#[inline(always)]
	fn region_weight(&self, p: Vec2) -> f32 {
		let d = self.region.sdf_with_noise(p, self.noise.as_ref());
		if d < -self.inner_radius {
			0.0
		} else if d > self.outer_radius {
			1.0
		} else {
			let t = (d + self.inner_radius) / (self.inner_radius + self.outer_radius);
			Self::smoothstep(t)
		}
	}

	pub fn branch_region(&self, noise: &RegionNoise) -> Self {
		let new_region = self.region.branch_region(noise);

		// use noise to get the new inner scale
		let new_inner_scale = (0.9
			+ noise.sample_fbm_double_peak(self.inner_scale, -self.inner_scale, 0.3, 0.05))
		.abs();

		let new_inner_offset =
			noise.sample_fbm_double_peak(self.inner_offset, -self.inner_offset, 1.0, 0.05);

		let new_inner_radius = self.inner_radius
			+ noise.sample_fbm_double_peak(
				self.inner_radius,
				-self.inner_radius,
				self.inner_radius,
				0.05,
			);

		let new_outer_radius = self.outer_radius
			+ noise.sample_fbm_double_peak(
				self.outer_radius,
				-self.outer_radius,
				self.outer_radius,
				0.05,
			);

		Self {
			region: new_region,
			inner_scale: new_inner_scale,
			inner_offset: new_inner_offset,
			inner_radius: new_inner_radius,
			outer_radius: new_outer_radius,
			noise: Some(noise.clone()),
		}
	}
}

impl ElevationModulation for RegionAffineModulation {
	fn modify_elevation(
		&self,
		_perlin_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		let p = Vec2::new(x, z);
		let w = self.region_weight(p);

		// Smooth blend between inside and outside values
		let a = self.inner_scale + (1.0 - self.inner_scale) * w;
		let b = self.inner_offset * (1.0 - w);

		a * elevation + b
	}
}
