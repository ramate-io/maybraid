use crate::region::{Region2D, RegionNoise};
use crate::{ElevationModulation, TerrainSdf};
use bevy::prelude::*;

/// Rounds the terrain height to the nearest unit amount.
#[derive(Debug, Clone)]
pub struct RegionGradingModulation {
	/// The region to round.
	pub region: Region2D,
	/// The start point of the grading.
	pub start: Vec2,
	/// The elevation at the start point.
	pub start_elevation: f32,
	/// The end point
	pub end: Vec2,
	/// The elevation at the end point.
	pub end_elevation: f32,
	/// Optional noise for perturbing the region boundary
	pub noise: Option<RegionNoise>,
	/// The inner radius of the region.
	pub inner_radius: f32,
	/// The outer radius of the region.
	pub outer_radius: f32,
}

impl RegionGradingModulation {
	pub fn new(
		region: Region2D,
		start: Vec2,
		start_elevation: f32,
		end: Vec2,
		end_elevation: f32,
		noise: Option<RegionNoise>,
		inner_radius: f32,
		outer_radius: f32,
	) -> Self {
		Self {
			region,
			start,
			start_elevation,
			end,
			end_elevation,
			noise,
			inner_radius,
			outer_radius,
		}
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
}

impl ElevationModulation for RegionGradingModulation {
	fn modify_elevation(
		&self,
		_perlin_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		// compute the distance to the start and end points
		let distance_to_start = (Vec2::new(x, z) - self.start).length();
		let distance_to_end = (Vec2::new(x, z) - self.end).length();

		// compute the fraction of the total distance which is the progress from the start
		let progress = distance_to_start / (distance_to_start + distance_to_end);

		// interpolate the elevation between the start and end points using the progress
		let interpolated_elevation =
			self.start_elevation + (self.end_elevation - self.start_elevation) * progress;

		// weighted elevation and the interpolated elevation
		let weight = self.region_weight(Vec2::new(x, z));

		weight * elevation + (1.0 - weight) * interpolated_elevation
	}
}
