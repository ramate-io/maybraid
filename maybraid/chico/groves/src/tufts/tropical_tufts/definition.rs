//! [`TropicalTuftsDefinition`] — well-known tuft grove ([RFC-183 §3.4.4.5], [#305](https://github.com/ramate-io/maybraid/issues/305)).

use bevy_math::Vec2;
use procedural_common::UnitRange;

use super::TropicalTuftsCell;
use crate::grove::{CellGrove, GroveDistribution, GrovePlacementRanges};

/// Authored Tropical Tufts grove definition.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalTuftsDefinition {
	cell_extent_xz: Vec2,
	placement: GrovePlacementRanges,
	distribution: GroveDistribution<TropicalTuftsCell>,
}

impl Default for TropicalTuftsDefinition {
	fn default() -> Self {
		Self::new()
	}
}

impl TropicalTuftsDefinition {
	pub const AUTHORED_CELL_EXTENT_XZ: Vec2 = Vec2::splat(3.25);

	pub const PLACEMENT_RANGES: GrovePlacementRanges = GrovePlacementRanges::new(
		UnitRange::new(0.85, 1.15),
		UnitRange::new(0.0, 1.0),
		UnitRange::new(0.10, 0.30),
		UnitRange::new(0.04, 0.12),
	);

	pub const VARIANT_WEIGHTS_CLI: &str = "29.4,2,1.5,1,0.75,0.35";

	pub fn new() -> Self {
		Self {
			cell_extent_xz: Self::AUTHORED_CELL_EXTENT_XZ,
			placement: Self::PLACEMENT_RANGES,
			distribution: TropicalTuftsCell::grove_distribution(),
		}
	}

	pub fn with_cell_extent_xz(mut self, cell_extent_xz: Vec2) -> Self {
		self.cell_extent_xz = cell_extent_xz.max(Vec2::splat(0.1));
		self
	}

	pub fn with_variant_weights(
		mut self,
		overrides: &crate::grove::VariantWeightOverrides,
	) -> Result<Self, String> {
		overrides.apply_to(&mut self.distribution)?;
		Ok(self)
	}

	pub fn cell_extent_xz_default() -> Vec2 {
		Self::AUTHORED_CELL_EXTENT_XZ
	}
}

impl CellGrove for TropicalTuftsDefinition {
	type Variant = TropicalTuftsCell;

	fn cell_extent_xz(&self) -> Vec2 {
		self.cell_extent_xz
	}

	fn placement_ranges(&self) -> GrovePlacementRanges {
		self.placement
	}

	fn distribution(&self) -> &GroveDistribution<Self::Variant> {
		&self.distribution
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{
		ForestGroveBiases, Grove, GroveCellOutcome, GroveNoiseConfig, PlacementConstraints,
		TerrainSample,
	};
	use anyhow::Result;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use gimme_gen::Cell;

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

	fn test_extent() -> crate::grove::GroveExtent {
		crate::grove::GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0))
	}

	fn assembled_grove() -> Grove<TropicalTuftsDefinition> {
		Grove::assemble(
			TropicalTuftsDefinition::new(),
			ForestGroveBiases { bucket_mean_shift: 0.0, ..Default::default() },
			GroveNoiseConfig::default(),
			Vec3::ZERO,
		)
	}

	#[test]
	fn authored_definition_matches_rfc() -> Result<()> {
		let grove = TropicalTuftsDefinition::new();
		assert_eq!(grove.cell_extent_xz(), TropicalTuftsDefinition::AUTHORED_CELL_EXTENT_XZ);
		let placement = grove.placement_ranges();
		assert_eq!(placement.scale, UnitRange::new(0.85, 1.15));
		assert_eq!(placement.offset, UnitRange::new(0.0, 1.0));
		assert_eq!(placement.noise_amplitude, UnitRange::new(0.10, 0.30));
		assert_eq!(placement.noise_frequency, UnitRange::new(0.04, 0.12));
		Ok(())
	}

	#[test]
	fn distribution_bucket_count_and_weights() -> Result<()> {
		let dist = TropicalTuftsCell::grove_distribution();
		assert_eq!(dist.len(), 6);
		assert!(dist.buckets[0].weight > 25.0);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].weight, 1.5);
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].weight, 0.75);
		assert_eq!(dist.buckets[5].weight, 0.35);
		Ok(())
	}

	#[test]
	fn bright_tuft_palette_matches_rfc() -> Result<()> {
		let dist = TropicalTuftsCell::grove_distribution();
		let Some(TropicalTuftsCell::BrightTuft(bucket)) = dist.buckets[1].item.as_ref() else {
			anyhow::bail!("expected BrightTuft bucket");
		};
		assert_eq!(bucket.palette_mix.slots.len(), 3);
		assert_eq!(bucket.palette_mix.slots[0].start, crate::grove::PaletteColor("bright_green"));
		Ok(())
	}

	#[test]
	fn placed_item_carries_geometry() -> Result<()> {
		let dist = TropicalTuftsCell::grove_distribution();
		let Some(TropicalTuftsCell::BrightTuft(bucket)) = dist.buckets[1].item.as_ref() else {
			anyhow::bail!("expected BrightTuft bucket");
		};
		assert_eq!(bucket.item.height, UnitRange::new(0.25, 0.50));
		Ok(())
	}

	#[test]
	fn palm_bucket_carries_geometry() -> Result<()> {
		let dist = TropicalTuftsCell::grove_distribution();
		let Some(TropicalTuftsCell::SmallPalmBush(bucket)) = dist.buckets[4].item.as_ref() else {
			anyhow::bail!("expected SmallPalmBush bucket");
		};
		assert_eq!(bucket.item.frond_count, 4..=7);
		Ok(())
	}

	#[test]
	fn assemble_selects_placed_variant() -> Result<()> {
		let grove = assembled_grove();
		let terrain = FlatTerrain { elevation: 0.4, steepness: 0.1 };
		let sampled = grove.placement_ranges().sample_at(
			grove.biases(),
			grove.noise(),
			Vec3::new(5.0, 0.0, 5.0),
		);
		let position = sampled.position_in(&test_cell());
		let outcome = grove.prepared().select_at_with_start(1, position, sampled, &terrain);
		assert!(matches!(outcome, GroveCellOutcome::Placed { .. }));
		Ok(())
	}

	#[test]
	fn none_bucket_yields_empty() -> Result<()> {
		let mut grove = TropicalTuftsDefinition::new();
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
			&test_extent(),
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
		let extent = test_extent();
		let a = grove.select_cell(&cell, &extent, &terrain);
		let b = grove.select_cell(&cell, &extent, &terrain);
		assert_eq!(a, b);
		Ok(())
	}
}
