//! Terrain sampling for placement constraints ([RFC-183 3.4.2.4]).

use bevy_math::Vec3;

/// Normalized elevation and steepness at world positions.
pub trait TerrainSample {
	fn elevation_at(&self, position: Vec3) -> f32;
	fn steepness_at(&self, position: Vec3) -> f32;
}

impl TerrainSample for f32 {
	fn elevation_at(&self, _position: Vec3) -> f32 {
		*self
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		*self
	}
}

/// Uniform terrain sample for CLI previews and isolation tests.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(next_help_heading = "Terrain"))]
pub struct FlatTerrainSample {
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.0))]
	pub elevation: f32,
	#[cfg_attr(feature = "render", arg(long, default_value_t = 0.1))]
	pub steepness: f32,
}

impl Default for FlatTerrainSample {
	fn default() -> Self {
		Self { elevation: 0.0, steepness: 0.1 }
	}
}

impl TerrainSample for FlatTerrainSample {
	fn elevation_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		self.steepness
	}
}
