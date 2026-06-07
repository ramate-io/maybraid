//! Terrain sampling for placement constraints ([RFC-183 3.4.2.4]).

use bevy_math::Vec3;

/// Normalized elevation and steepness at world positions.
pub trait TerrainSample {
	fn elevation_at(&self, position: Vec3) -> f32;
	fn steepness_at(&self, position: Vec3) -> f32;
}
