//! Orchard — high-density cultivated Storybook Tree upper-canopy grove
//! ([RFC-183 §3.4.7.7], [#353](https://github.com/ramate-io/maybraid/issues/353)).
//!
//! Compact fruiting and pale-bloom storybook forms on low-slope terrain with tight cell offset.
//! Forest-layer attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};


/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);

/// Authored Orchard grove definition.
///
/// Cell footprint sits at the RFC midpoint (`11.0` m). Placements stay on cell centroids with only
/// ±`0.5` m horizontal jitter so the grove reads as regular tended rows.
pub fn definition() -> GroveDefinition<OrchardCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(11.0),
		placement: GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(-0.5, 0.5)),
		distribution: OrchardCell::distribution(),
	}
}

/// Ordered orchard varietals ([RFC-183 §3.4.7.7]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchardCell {
	FruitingStorybook,
	PaleBloomStorybook,
}

/// Typed authored geometry for one orchard varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrchardItem {
	Storybook(&'static OrchardStorybook),
}

/// Authored geometry ranges for one cultivated Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct OrchardStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const FRUITING_STORYBOOK: OrchardStorybook = OrchardStorybook {
	height: UnitRange::new(5.0, 10.0),
	stalk_radius: UnitRange::new(0.22, 0.44),
	canopy_spread: UnitRange::new(1.8, 4.2),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const PALE_BLOOM_STORYBOOK: OrchardStorybook = OrchardStorybook {
	height: UnitRange::new(5.0, 9.0),
	stalk_radius: UnitRange::new(0.20, 0.38),
	canopy_spread: UnitRange::new(1.6, 3.8),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const FRUITING_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("orchard_bark", "brown_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const FRUITING_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("fresh_green", "light_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const PALE_BLOOM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("orchard_bark", "gray_brown"),
	PaletteSlot::new("tan_bark", "brown_bark"),
]);

const PALE_BLOOM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("pale_blossom", "fresh_green"),
	PaletteSlot::new("light_green", "yellow_green"),
]);

/// Explicit `None` weight paired with placed weights so ~`95%` of cells receive a tree.
const CULTIVATED_EMPTY_WEIGHT: f32 = 2.25 / 19.0;

impl OrchardCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.25`; the `None` weight of `2.25 / 19` yields a `~0.95` placed share
	/// for regular tended-row planting.
	pub fn distribution() -> GroveDistribution<Self> {
		let fruiting =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.30));
		let pale_bloom =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.28));
		GroveDistribution::new(vec![
			GroveBucket::none(CULTIVATED_EMPTY_WEIGHT),
			GroveBucket::placed(1.5, fruiting, Self::FruitingStorybook),
			GroveBucket::placed(0.75, pale_bloom, Self::PaleBloomStorybook),
		])
	}

	pub fn item(self) -> OrchardItem {
		match self {
			Self::FruitingStorybook => OrchardItem::Storybook(&FRUITING_STORYBOOK),
			Self::PaleBloomStorybook => OrchardItem::Storybook(&PALE_BLOOM_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::FruitingStorybook => FRUITING_STICK_MIX,
			Self::PaleBloomStorybook => PALE_BLOOM_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::FruitingStorybook => FRUITING_CANOPY_MIX,
			Self::PaleBloomStorybook => PALE_BLOOM_CANOPY_MIX,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{
		FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent,
	};
	use anyhow::Result;
	use bevy_math::Vec3;
	use gimme_gen::Cell;
	use procedural_common::NoiseParams;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = OrchardCell::distribution();
		assert_eq!(dist.len(), 3);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, CULTIVATED_EMPTY_WEIGHT);
		assert_eq!(dist.buckets[1].item, Some(OrchardCell::FruitingStorybook));
		assert_eq!(dist.buckets[1].weight, 1.5);
		assert_eq!(dist.buckets[2].item, Some(OrchardCell::PaleBloomStorybook));
		assert_eq!(dist.buckets[2].weight, 0.75);
		Ok(())
	}

	#[test]
	fn placed_share_targets_cultivated_fill() -> Result<()> {
		let dist = OrchardCell::distribution();
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
	fn geometry_follows_authored_bands() -> Result<()> {
		let OrchardItem::Storybook(fruiting) = OrchardCell::FruitingStorybook.item();
		assert_eq!(fruiting.height, UnitRange::new(5.0, 10.0));
		assert_eq!(fruiting.canopy_density, MODERATE_CANOPY_DENSITY);

		let OrchardItem::Storybook(pale) = OrchardCell::PaleBloomStorybook.item();
		assert_eq!(pale.height, UnitRange::new(5.0, 9.0));
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = OrchardCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let fruiting = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(OrchardCell::FruitingStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing fruiting bucket"))?;
		assert_eq!(fruiting.constraints.steepness.end, 0.30);

		let pale = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(OrchardCell::PaleBloomStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing pale bloom bucket"))?;
		assert_eq!(pale.constraints.steepness.end, 0.28);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_fruiting_but_allows_pale_on_gentler_band() -> Result<()> {
		let prepared =
			OrchardCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let gentle = FlatTerrainSample { elevation: 0.40, steepness: 0.25 };
		let fruiting_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&gentle,
		);
		match fruiting_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, OrchardCell::FruitingStorybook);
			}
			other => anyhow::bail!("expected FruitingStorybook on gentle slope, got {other:?}"),
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.32 };
		let steep_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep,
		);
		match steep_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, OrchardCell::FruitingStorybook);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [OrchardCell::FruitingStorybook, OrchardCell::PaleBloomStorybook] {
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
	fn placement_uses_tight_centroid_offset_and_uniform_scale() -> Result<()> {
		let def = definition();
		assert_eq!(def.placement.offset, UnitRange::new(-0.5, 0.5));
		assert_eq!(def.placement.scale, UnitRange::new(1.0, 1.0));
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.10 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
