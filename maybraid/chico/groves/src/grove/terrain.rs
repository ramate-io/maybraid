//! World sampling and per-variant placement constraints ([RFC-183 3.4.1.5–6, 3.4.2.4]).

use bevy_math::bounding::{Aabb3d, BoundingVolume};
use bevy_math::{Vec3, Vec3A};
use procedural_common::UnitRange;

/// World-space height, steepness, and placement exclusion at positions.
///
/// [`Self::height_at`] is world metres (plant Y). Constraint bands stay authored on
/// buckets but are not evaluated here — normalization is forest/region policy.
pub trait GroveWorldSample {
	/// Surface height in world metres at `position` (XZ).
	fn height_at(&self, position: Vec3) -> f32;
	fn steepness_at(&self, position: Vec3) -> f32;

	/// Axis-aligned regions where grove items must not be placed.
	fn exclusion_zones(&self) -> &[Aabb3d] {
		&[]
	}

	/// Whether a grove item may occupy `position` on this sample layer.
	fn allows_placement_at(&self, position: Vec3) -> bool {
		!point_in_any_aabb(position, self.exclusion_zones())
	}
}

/// Uniform world sample for CLI previews and isolation tests.
///
/// `elevation` is a constant world-metre height (CLI flag kept for existing scripts).
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

/// World-metre height from a function (Durham adapter, tests).
pub struct FnHeightSample<F>(pub F);

impl<F: Fn(Vec3) -> f32> GroveWorldSample for FnHeightSample<F> {
	fn height_at(&self, position: Vec3) -> f32 {
		(self.0)(position)
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		0.0
	}
}

impl GroveWorldSample for FlatTerrainSample {
	fn height_at(&self, _position: Vec3) -> f32 {
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

fn point_in_any_aabb(position: Vec3, zones: &[Aabb3d]) -> bool {
	let point = Aabb3d::from_min_max(Vec3A::from(position), Vec3A::from(position));
	zones.iter().any(|zone| zone.contains(&point))
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

	#[test]
	fn flat_sample_allows_all_placements() -> Result<()> {
		let sample = FlatTerrainSample::default();
		assert!(sample.allows_placement_at(Vec3::ZERO));
		assert!(sample.allows_placement_at(Vec3::new(100.0, 0.0, -50.0)));
		Ok(())
	}

	#[test]
	fn exclusion_zones_block_placement() -> Result<()> {
		struct SampleWithExclusion {
			zones: Vec<Aabb3d>,
		}

		impl GroveWorldSample for SampleWithExclusion {
			fn height_at(&self, _position: Vec3) -> f32 {
				0.5
			}

			fn steepness_at(&self, _position: Vec3) -> f32 {
				0.1
			}

			fn exclusion_zones(&self) -> &[Aabb3d] {
				&self.zones
			}
		}

		let sample =
			SampleWithExclusion { zones: vec![Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE)] };
		assert!(!sample.allows_placement_at(Vec3::new(0.5, 0.5, 0.5)));
		assert!(sample.allows_placement_at(Vec3::new(2.0, 0.0, 0.0)));
		Ok(())
	}
}
