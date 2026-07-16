use crate::terrain::region::{Region2D, RegionNoise};
use crate::terrain::sdf::{ElevationModulation, TerrainSdf};
use bevy::prelude::*;

/// Rounds the terrain height to the nearest unit amount inside a region.
#[derive(Debug, Clone)]
pub struct RegionRoundingModulation {
	pub region: Region2D,
	pub nearest: f32,
	pub noise: Option<RegionNoise>,
	pub inner_radius: f32,
	pub outer_radius: f32,
}

impl RegionRoundingModulation {
	pub fn new(
		region: Region2D,
		nearest: f32,
		noise: Option<RegionNoise>,
		inner_radius: f32,
		outer_radius: f32,
	) -> Self {
		Self { region, nearest, noise, inner_radius, outer_radius }
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

impl ElevationModulation for RegionRoundingModulation {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		let rounded = (elevation / self.nearest).round() * self.nearest;
		let weight = self.region_weight(Vec2::new(x, z));
		weight * elevation + (1.0 - weight) * rounded
	}
}
