//! Terrain sampling and per-variant placement constraints ([RFC-183 3.4.1.5–6, 3.4.2.4]).

use bevy_math::Vec3;
use procedural_common::UnitRange;

/// Normalized elevation and steepness at world positions.
pub trait TerrainSample {
	fn elevation_at(&self, position: Vec3) -> f32;
	fn steepness_at(&self, position: Vec3) -> f32;
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

/// Elevation and steepness ranges attached to each bucketed variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementConstraints {
	pub elevation: UnitRange,
	pub steepness: UnitRange,
}

impl PlacementConstraints {
	pub const fn new(elevation: UnitRange, steepness: UnitRange) -> Self {
		Self { elevation, steepness }
	}

	pub const UNCONSTRAINED: Self =
		Self { elevation: UnitRange::new(0.0, 1.0), steepness: UnitRange::new(0.0, 1.0) };

	/// Whether normalized elevation and steepness satisfy this variant's half-open ranges.
	pub fn allows(&self, elevation: f32, steepness: f32) -> bool {
		scalar_in_half_open_range(elevation, self.elevation)
			&& scalar_in_half_open_range(steepness, self.steepness)
	}
}

fn scalar_in_half_open_range(value: f32, range: UnitRange) -> bool {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	value >= lo && value < hi
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn constraints_use_half_open_ranges() -> Result<()> {
		let constraints =
			PlacementConstraints::new(UnitRange::new(0.2, 0.6), UnitRange::new(0.0, 0.3));
		assert!(constraints.allows(0.5, 0.1));
		assert!(constraints.allows(0.2, 0.0));
		assert!(!constraints.allows(0.6, 0.1), "upper bound is exclusive");
		assert!(!constraints.allows(0.5, 0.9));
		Ok(())
	}
}
