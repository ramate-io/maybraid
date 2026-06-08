//! [`BraidGrassGrove`] — well-known understory grove ([RFC-183 §3.4.5.1], [#306](https://github.com/ramate-io/maybraid/issues/306)).

use procedural_common::UnitRange;

use crate::braid_grass::BraidGrassCell;
use crate::grove::{CellGrove, GroveDistribution, GroveParamRanges};

/// Authored Braid Grass grove definition.
#[derive(Debug, Clone, PartialEq)]
pub struct BraidGrassGrove {
	ranges: GroveParamRanges,
	distribution: GroveDistribution<BraidGrassCell>,
}

impl Default for BraidGrassGrove {
	fn default() -> Self {
		Self::new()
	}
}

impl BraidGrassGrove {
	/// RFC §3.4.5.1 grove-level parameter ranges.
	pub const AUTHORED_RANGES: GroveParamRanges = GroveParamRanges::new(
		UnitRange::new(2.5, 6.0),
		UnitRange::new(0.85, 1.15),
		UnitRange::new(0.35, 0.75),
		UnitRange::new(0.0, 1.0),
		UnitRange::new(0.10, 0.35),
		UnitRange::new(0.03, 0.10),
	);

	pub fn new() -> Self {
		Self {
			ranges: Self::AUTHORED_RANGES,
			distribution: BraidGrassCell::grove_distribution(),
		}
	}

	pub fn with_ranges(mut self, ranges: GroveParamRanges) -> Self {
		self.ranges = ranges;
		self
	}
}

impl CellGrove for BraidGrassGrove {
	type Variant = BraidGrassCell;

	fn param_ranges(&self) -> GroveParamRanges {
		self.ranges
	}

	fn distribution(&self) -> &GroveDistribution<Self::Variant> {
		&self.distribution
	}
}

#[cfg(test)]
mod tests {
	use std::mem;

	use super::*;
	use crate::braid_grass::BraidGrass;
	use crate::grove::{
		candidate_position, sample_cell_params, ForestGroveBiases, Grove, GroveCellOutcome,
		GroveNoiseConfig, PlacementConstraints, TerrainSample,
	};
	use anyhow::Result;
	use bevy_math::Vec3;
	use bevy_math::bounding::Aabb3d;
	use gimme_gen::Cell;
	use procedural_common::UnitRange;

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

	fn test_cell() -> Cell {
		Cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0)))
	}

	fn assembled_grove() -> Grove<BraidGrassGrove> {
		Grove::assemble(
			BraidGrassGrove::new(),
			ForestGroveBiases { bucket_mean_shift: 0.0, ..Default::default() },
			GroveNoiseConfig::default(),
			Vec3::ZERO,
		)
	}

	#[test]
	fn authored_ranges_match_rfc() -> Result<()> {
		let grove = BraidGrassGrove::new();
		let ranges = grove.param_ranges();
		assert_eq!(ranges.cell_size, UnitRange::new(2.5, 6.0));
		assert_eq!(ranges.density, UnitRange::new(0.35, 0.75));
		assert_eq!(ranges.offset, UnitRange::new(0.0, 1.0));
		assert_eq!(ranges.noise_amplitude, UnitRange::new(0.10, 0.35));
		assert_eq!(ranges.noise_frequency, UnitRange::new(0.03, 0.10));
		assert_eq!(ranges.scale, UnitRange::new(0.85, 1.15));
		Ok(())
	}

	#[test]
	fn distribution_bucket_count_and_weights() -> Result<()> {
		let dist = BraidGrassCell::grove_distribution();
		assert_eq!(dist.len(), 5);
		assert_eq!(dist.buckets[0].weight, 2.5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].weight, 1.0);
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].weight, 0.5);
		Ok(())
	}

	#[test]
	fn placed_item_carries_geometry() -> Result<()> {
		let dist = BraidGrassCell::grove_distribution();
		let Some(BraidGrassCell::DeepGreenBlade(bucket)) = dist.buckets[1].item.as_ref() else {
			anyhow::bail!("expected DeepGreenBlade bucket");
		};
		assert_eq!(bucket.item.height, UnitRange::new(1.0, 2.2));
		Ok(())
	}

	#[test]
	fn braid_grass_geometry_in_bucket() -> Result<()> {
		assert!(mem::size_of::<BraidGrass>() > 0);
		let dist = BraidGrassCell::grove_distribution();
		let Some(BraidGrassCell::DeepGreenBlade(bucket)) = dist.buckets[1].item.as_ref() else {
			anyhow::bail!("expected DeepGreenBlade variant");
		};
		assert_eq!(bucket.item.blade_count, 12..=28);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		let grove = assembled_grove();
		let terrain = FlatTerrain { elevation: 0.3, steepness: 0.35 };
		let sampled = sample_cell_params(
			&grove.param_ranges(),
			grove.biases(),
			grove.noise(),
			Vec3::new(5.0, 0.0, 5.0),
		);
		let position = candidate_position(&test_cell(), sampled.offset);
		// Jungle (index 3) rejects steepness 0.35; first-fit wraps to RedEdge (index 4).
		let outcome = grove
			.prepared()
			.select_at_with_start(3, position, sampled, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert!(matches!(variant, BraidGrassCell::RedEdgeBlade(_)));
			}
			other => anyhow::bail!("expected RedEdgeBlade fallback, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn assemble_selects_placed_variant() -> Result<()> {
		let grove = assembled_grove();
		let terrain = FlatTerrain { elevation: 0.4, steepness: 0.1 };
		let sampled = sample_cell_params(
			&grove.param_ranges(),
			grove.biases(),
			grove.noise(),
			Vec3::new(5.0, 0.0, 5.0),
		);
		let position = candidate_position(&test_cell(), sampled.offset);
		// None bucket weight dominates random throw; pin start to a placed bucket.
		let outcome = grove
			.prepared()
			.select_at_with_start(1, position, sampled, &terrain);
		assert!(matches!(outcome, GroveCellOutcome::Placed { .. }));
		Ok(())
	}

	#[test]
	fn none_bucket_yields_empty() -> Result<()> {
		let mut grove = BraidGrassGrove::new();
		let mut dist = GroveDistribution::new();
		dist.push(crate::grove::GroveBucket {
			weight: 100.0,
			constraints: PlacementConstraints::UNCONSTRAINED,
			item: None,
		});
		grove.distribution = dist;
		let assembled = Grove::assemble(
			grove,
			ForestGroveBiases { bucket_mean_shift: 0.0, ..Default::default() },
			GroveNoiseConfig::default(),
			Vec3::ZERO,
		);
		let outcome = assembled.select_cell(
			&test_cell(),
			&FlatTerrain { elevation: 0.5, steepness: 0.1 },
		);
		assert!(matches!(outcome, GroveCellOutcome::Empty { .. }));
		Ok(())
	}

	#[test]
	fn deterministic_replay() -> Result<()> {
		let grove = assembled_grove();
		let cell = test_cell();
		let terrain = FlatTerrain { elevation: 0.35, steepness: 0.15 };
		let a = grove.select_cell(&cell, &terrain);
		let b = grove.select_cell(&cell, &terrain);
		assert_eq!(a, b);
		Ok(())
	}
}
