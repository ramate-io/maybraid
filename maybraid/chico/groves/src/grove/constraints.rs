//! Per-variant placement constraints ([RFC-183 3.4.1.5–6]).

use procedural_common::UnitRange;

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

	pub const UNCONSTRAINED: Self = Self {
		elevation: UnitRange::new(0.0, 1.0),
		steepness: UnitRange::new(0.0, 1.0),
	};

	/// Whether normalized elevation and steepness satisfy this bucket's half-open ranges.
	pub fn allows(&self, elevation: f32, steepness: f32) -> bool {
		Self::scalar_in_half_open_range(elevation, self.elevation)
			&& Self::scalar_in_half_open_range(steepness, self.steepness)
	}

	fn scalar_in_half_open_range(value: f32, range: UnitRange) -> bool {
		let lo = range.start.min(range.end);
		let hi = range.start.max(range.end);
		value >= lo && value < hi
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{distribution::GroveBucket, terrain::TerrainSample};
	use anyhow::Result;
	use bevy_math::Vec3;

	struct FlatTerrain {
		elevation: f32,
		steepness: f32,
	}

	impl TerrainSample for FlatTerrain {
		fn elevation_at(&self, _position: Vec3) -> f32 {
			self.elevation
		}

		fn steepness_at(&self, _position: Vec3) -> f32 {
			self.steepness
		}
	}

	#[test]
	fn none_bucket_always_valid() -> Result<()> {
		let bucket = GroveBucket::<()> {
			weight: 1.0,
			constraints: PlacementConstraints::new(UnitRange::new(0.9, 1.0), UnitRange::new(0.0, 0.1)),
			item: None,
		};
		let terrain = FlatTerrain { elevation: 0.0, steepness: 0.99 };
		assert!(bucket.valid_at(Vec3::ZERO, &terrain));
		Ok(())
	}

	#[test]
	fn variant_must_match_constraints() -> Result<()> {
		let bucket = GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::new(UnitRange::new(0.2, 0.6), UnitRange::new(0.0, 0.3)),
			item: Some(42_u32),
		};
		let terrain = FlatTerrain { elevation: 0.5, steepness: 0.1 };
		assert!(bucket.valid_at(Vec3::ZERO, &terrain));
		let steep = FlatTerrain { elevation: 0.5, steepness: 0.9 };
		assert!(!bucket.valid_at(Vec3::ZERO, &steep));
		Ok(())
	}
}
