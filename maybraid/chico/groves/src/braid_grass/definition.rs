//! [`BraidGrassDefinition`] — well-known understory grove ([RFC-183 §3.4.5.1], [#306](https://github.com/ramate-io/maybraid/issues/306)).

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::braid_grass::BraidGrassCell;
use crate::grove::{CellGrove, GroveDistribution, GrovePlacementRanges};

/// Authored Braid Grass grove definition.
#[derive(Debug, Clone, PartialEq)]
pub struct BraidGrassDefinition {
	cell_extent_xz: Vec2,
	placement: GrovePlacementRanges,
	distribution: GroveDistribution<BraidGrassCell>,
}

impl Default for BraidGrassDefinition {
	fn default() -> Self {
		Self::new()
	}
}

impl BraidGrassDefinition {
	/// RFC §3.4.5.1 authored cell footprint (metres on X and Z).
	///
	/// Forest gridding may choose any span inside the RFC `2.5..6.0` band; this is the
	/// definition default used by playground previews until a forest pass supplies cells.
	pub const AUTHORED_CELL_EXTENT_XZ: Vec2 = Vec2::splat(4.25);

	/// Per-cell placement ranges from RFC §3.4.5.1.
	///
	/// Offset uses a wider overspill band than the RFC's nominal ±1 m so biased sampling plus
	/// noise still reaches meaningful horizontal variety; [`GroveExtent`] validation keeps the
	/// grove LOD unit bounded.
	pub const PLACEMENT_RANGES: GrovePlacementRanges = GrovePlacementRanges::new(
		UnitRange::new(0.85, 1.15),
		UnitRange::new(-3.0, 3.0),
		UnitRange::new(0.10, 0.35),
		UnitRange::new(0.03, 0.10),
	);

	pub fn new() -> Self {
		Self {
			cell_extent_xz: Self::AUTHORED_CELL_EXTENT_XZ,
			placement: Self::PLACEMENT_RANGES,
			distribution: BraidGrassCell::grove_distribution(),
		}
	}

	pub fn with_cell_extent_xz(mut self, cell_extent_xz: Vec2) -> Self {
		self.cell_extent_xz = cell_extent_xz.max(Vec2::splat(0.1));
		self
	}

	pub fn with_placement_ranges(mut self, placement: GrovePlacementRanges) -> Self {
		self.placement = placement;
		self
	}

	pub fn with_variant_weights(
		mut self,
		overrides: &crate::grove::VariantWeightOverrides,
	) -> Result<Self, String> {
		overrides.apply_to(&mut self.distribution)?;
		Ok(self)
	}

	/// Authored bucket weights for CLI help (`None` bucket, then macro declaration order).
	pub const VARIANT_WEIGHTS_CLI: &str = "2.5,2,1,1,0.5";

	/// Default cell footprint for gridding this grove.
	pub fn cell_extent_xz_default() -> Vec2 {
		Self::AUTHORED_CELL_EXTENT_XZ
	}
}

impl CellGrove for BraidGrassDefinition {
	type Variant = BraidGrassCell;

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
	use std::mem;

	use super::*;
	use crate::braid_grass::BraidGrassClump;
	use crate::grove::{
		ForestGroveBiases, Grove, GroveCellOutcome, GroveNoiseConfig, PlacementConstraints,
		TerrainSample,
	};
	use anyhow::Result;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
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

	fn test_extent() -> crate::grove::GroveExtent {
		crate::grove::GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0))
	}

	fn assembled_grove() -> Grove<BraidGrassDefinition> {
		Grove::assemble(
			BraidGrassDefinition::new(),
			ForestGroveBiases { bucket_mean_shift: 0.0, ..Default::default() },
			GroveNoiseConfig::default(),
			Vec3::ZERO,
		)
	}

	#[test]
	fn authored_definition_matches_rfc() -> Result<()> {
		let grove = BraidGrassDefinition::new();
		assert_eq!(grove.cell_extent_xz(), BraidGrassDefinition::AUTHORED_CELL_EXTENT_XZ);
		let placement = grove.placement_ranges();
		assert_eq!(placement.offset, UnitRange::new(-3.0, 3.0));
		assert_eq!(placement.noise_amplitude, UnitRange::new(0.10, 0.35));
		assert_eq!(placement.noise_frequency, UnitRange::new(0.03, 0.10));
		assert_eq!(placement.scale, UnitRange::new(0.85, 1.15));
		Ok(())
	}

	#[test]
	fn deep_green_palette_matches_rfc() -> Result<()> {
		let dist = BraidGrassCell::grove_distribution();
		let Some(BraidGrassCell::DeepGreenBlade(bucket)) = dist.buckets[1].item.as_ref() else {
			anyhow::bail!("expected DeepGreenBlade bucket");
		};
		assert_eq!(bucket.palette_mix.slots.len(), 3);
		assert_eq!(bucket.palette_mix.slots[0].start, crate::grove::PaletteColor("deep_green"));
		assert_eq!(bucket.palette_mix.slots[0].end, crate::grove::PaletteColor("wet_green"));
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
		assert!(mem::size_of::<BraidGrassClump>() > 0);
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
		let sampled = grove.placement_ranges().sample_at(
			grove.biases(),
			grove.noise(),
			Vec3::new(5.0, 0.0, 5.0),
		);
		let position = sampled.position_in(&test_cell());
		// Jungle (index 3) rejects steepness 0.35; first-fit wraps to RedEdge (index 4).
		let outcome = grove.prepared().select_at_with_start(3, position, sampled, &terrain);
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
		let sampled = grove.placement_ranges().sample_at(
			grove.biases(),
			grove.noise(),
			Vec3::new(5.0, 0.0, 5.0),
		);
		let position = sampled.position_in(&test_cell());
		// None bucket weight dominates random throw; pin start to a placed bucket.
		let outcome = grove.prepared().select_at_with_start(1, position, sampled, &terrain);
		assert!(matches!(outcome, GroveCellOutcome::Placed { .. }));
		Ok(())
	}

	#[test]
	fn none_bucket_yields_empty() -> Result<()> {
		let mut grove = BraidGrassDefinition::new();
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
