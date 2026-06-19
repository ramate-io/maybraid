//! Vineyard — high-density cultivated Rory-trained vine upper-canopy grove
//! ([RFC-183 §3.4.7.8], [#355](https://github.com/ramate-io/maybraid/issues/355)).
//!
//! Low trained-vine rows with very tight cell offset and grape-like palettes. Forest-layer
//! attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{Vineyard, VineyardStd};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);

/// Authored Vineyard grove definition.
///
/// Cell footprint sits at the RFC midpoint (`4.5` m). Placements stay on cell centroids with only
/// ±`0.5` m horizontal jitter for regular vine rows.
pub fn definition() -> GroveDefinition<VineyardCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(4.5),
		placement: GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(-0.5, 0.5)),
		distribution: VineyardCell::distribution(),
	}
}

/// Ordered vineyard varietals ([RFC-183 §3.4.7.8]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VineyardCell {
	TrainedVineRory,
}

/// Typed authored geometry for one vineyard varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VineyardItem {
	Rory(&'static VineyardRory),
}

/// Authored geometry ranges for one trained-vine Rory form.
#[derive(Debug, Clone, PartialEq)]
pub struct VineyardRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const TRAINED_VINE_RORY: VineyardRory = VineyardRory {
	height: UnitRange::new(1.5, 3.0),
	stalk_radius: UnitRange::new(0.045, 0.090),
	canopy_spread: UnitRange::new(1.0, 2.4),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const VINE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("vine_bark", "red_brown"),
	PaletteSlot::new("weathered_bark", "gray_brown"),
]);

const VINE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("grape_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

/// Explicit `None` weight so ~`95%` of cells receive a vine (`0.05` empty vs `0.95` placed).
const CULTIVATED_EMPTY_WEIGHT: f32 = 0.05;
const CULTIVATED_PLACED_WEIGHT: f32 = 0.95;

impl VineyardCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// `None` weight `0.05` against placed weight `0.95` yields a `0.95` placed share for
	/// regular row planting.
	pub fn distribution() -> GroveDistribution<Self> {
		let trained_vine =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.34));
		GroveDistribution::new(vec![
			GroveBucket::none(CULTIVATED_EMPTY_WEIGHT),
			GroveBucket::placed(CULTIVATED_PLACED_WEIGHT, trained_vine, Self::TrainedVineRory),
		])
	}

	pub fn item(self) -> VineyardItem {
		match self {
			Self::TrainedVineRory => VineyardItem::Rory(&TRAINED_VINE_RORY),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		VINE_STICK_MIX
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		VINE_CANOPY_MIX
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
	use anyhow::Result;
	use bevy_math::Vec3;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = VineyardCell::distribution();
		assert_eq!(dist.len(), 2);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, CULTIVATED_EMPTY_WEIGHT);
		assert_eq!(dist.buckets[1].item, Some(VineyardCell::TrainedVineRory));
		assert_eq!(dist.buckets[1].weight, CULTIVATED_PLACED_WEIGHT);
		Ok(())
	}

	#[test]
	fn placed_share_targets_cultivated_fill() -> Result<()> {
		let dist = VineyardCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!(
			(0.94..=0.96).contains(&share),
			"placed share {share} outside cultivated ~95% target"
		);
		Ok(())
	}

	#[test]
	fn placement_uses_tight_centroid_offset_and_uniform_scale() -> Result<()> {
		let def = definition();
		assert_eq!(def.placement.offset, UnitRange::new(-0.5, 0.5));
		assert_eq!(def.placement.scale, UnitRange::new(1.0, 1.0));
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let VineyardItem::Rory(vine) = VineyardCell::TrainedVineRory.item();
		assert_eq!(vine.height, UnitRange::new(1.5, 3.0));
		assert_eq!(vine.canopy_spread, UnitRange::new(1.0, 2.4));
		assert_eq!(vine.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = VineyardCell::distribution();
		let vine = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(VineyardCell::TrainedVineRory))
			.ok_or_else(|| anyhow::anyhow!("missing trained vine bucket"))?;
		assert_eq!(vine.constraints.elevation.start, 0.0);
		assert_eq!(vine.constraints.elevation.end, 1.0);
		assert_eq!(vine.constraints.steepness.end, 0.34);
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [VineyardCell::TrainedVineRory] {
			for (palette, label) in
				[(cell.stick_palette_mix(), "stick"), (cell.canopy_palette_mix(), "canopy")]
			{
				let mut allowed = Vec::new();
				for slot in palette.slots {
					allowed.extend(slot.start.resolve());
					allowed.extend(slot.end.resolve());
				}
				assert!(!allowed.is_empty(), "unresolved {label} tokens for {cell:?}");
			}
		}
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.12 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
