//! Riparian General — moderate-density mixed river-corridor upper-canopy grove
//! ([RFC-183 §3.4.7.4], [#347](https://github.com/ramate-io/maybraid/issues/347)).
//!
//! Common Braid Oak and Storybook Tree forms with rare willow-like High Bush accents. Forest-layer
//! attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{RiparianGeneral, RiparianGeneralStd};

const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Flat sparse crown projection for willow-like High Bush forms.
const SPARSE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.42, 0.62);
const SPARSE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.52);

/// Authored Riparian General grove definition.
///
/// Cell footprint sits at the RFC midpoint (`16` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<RiparianGeneralCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(16.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-16.0, 16.0),
		),
		distribution: RiparianGeneralCell::distribution(),
	}
}

/// Ordered riparian-general varietals ([RFC-183 §3.4.7.4]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiparianGeneralCell {
	RiparianBraidOak,
	RiparianStorybook,
	RareRiparianHighBush,
}

/// Typed authored geometry for one riparian-general varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiparianGeneralItem {
	BraidOak(&'static RiparianGeneralBraidOak),
	Storybook(&'static RiparianGeneralStorybook),
	HighBush(&'static RiparianGeneralHighBush),
}

/// Authored geometry ranges for one Braid Oak form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianGeneralBraidOak {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianGeneralStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one willow-like Common High Bush form.
#[derive(Debug, Clone, PartialEq)]
pub struct RiparianGeneralHighBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

const RIPARIAN_BRAID_OAK: RiparianGeneralBraidOak = RiparianGeneralBraidOak {
	height: UnitRange::new(5.0, 15.0),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RIPARIAN_STORYBOOK: RiparianGeneralStorybook = RiparianGeneralStorybook {
	height: UnitRange::new(5.0, 15.0),
	stalk_radius: UnitRange::new(0.12, 0.28),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const RARE_RIPARIAN_HIGH_BUSH: RiparianGeneralHighBush = RiparianGeneralHighBush {
	height: UnitRange::new(5.0, 15.0),
	shoot_count: 5..=14,
	branch_depth: 2..=4,
	radial_strength: SPARSE_PROJECTION_RADIAL,
	vertical_bias: SPARSE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.12, 0.28),
};

const RIPARIAN_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_oak_bark", "dark_bark"),
	PaletteSlot::new("moss_bark", "gray_brown"),
]);

const RIPARIAN_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("river_green", "fresh_green"),
	PaletteSlot::new("deep_green", "yellow_green"),
]);

const RIPARIAN_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("broadleaf_bark", "wet_brown"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const RIPARIAN_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("river_green", "light_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const RIPARIAN_HIGH_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("willow_bark", "wet_brown"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const RIPARIAN_HIGH_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("fresh_green", "yellow_green"),
	PaletteSlot::new("river_green", "light_green"),
]);

impl RiparianGeneralCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `3.35`; the `None` weight of `7.4` puts the placed share at
	/// `3.35 / 10.75 ≈ 0.31`, mid RFC `DENSITY_RANGE` (`0.20..0.42`).
	pub fn distribution() -> GroveDistribution<Self> {
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.0, 0.42), UnitRange::new(0.0, 0.36));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.44));
		let high_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 0.38), UnitRange::new(0.0, 0.52));
		GroveDistribution::new(vec![
			GroveBucket::none(7.4),
			GroveBucket::placed(1.5, braid_oak, Self::RiparianBraidOak),
			GroveBucket::placed(1.5, storybook, Self::RiparianStorybook),
			GroveBucket::placed(0.35, high_bush, Self::RareRiparianHighBush),
		])
	}

	pub fn item(self) -> RiparianGeneralItem {
		match self {
			Self::RiparianBraidOak => RiparianGeneralItem::BraidOak(&RIPARIAN_BRAID_OAK),
			Self::RiparianStorybook => RiparianGeneralItem::Storybook(&RIPARIAN_STORYBOOK),
			Self::RareRiparianHighBush => RiparianGeneralItem::HighBush(&RARE_RIPARIAN_HIGH_BUSH),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::RiparianBraidOak => RIPARIAN_BRAID_OAK_STICK_MIX,
			Self::RiparianStorybook => RIPARIAN_STORYBOOK_STICK_MIX,
			Self::RareRiparianHighBush => RIPARIAN_HIGH_BUSH_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::RiparianBraidOak => RIPARIAN_BRAID_OAK_CANOPY_MIX,
			Self::RiparianStorybook => RIPARIAN_STORYBOOK_CANOPY_MIX,
			Self::RareRiparianHighBush => RIPARIAN_HIGH_BUSH_CANOPY_MIX,
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
	use procedural_common::NoiseParams;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = RiparianGeneralCell::distribution();
		assert_eq!(dist.len(), 4);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 7.4);
		assert_eq!(dist.buckets[1].item, Some(RiparianGeneralCell::RiparianBraidOak));
		assert_eq!(dist.buckets[1].weight, 1.5);
		assert_eq!(dist.buckets[2].item, Some(RiparianGeneralCell::RiparianStorybook));
		assert_eq!(dist.buckets[2].weight, 1.5);
		assert_eq!(dist.buckets[3].item, Some(RiparianGeneralCell::RareRiparianHighBush));
		assert_eq!(dist.buckets[3].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = RiparianGeneralCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.20..=0.42).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let RiparianGeneralItem::BraidOak(oak) = RiparianGeneralCell::RiparianBraidOak.item() else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.height, UnitRange::new(5.0, 15.0));
		assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

		let RiparianGeneralItem::Storybook(story) = RiparianGeneralCell::RiparianStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(5.0, 15.0));
		assert_eq!(story.canopy_density, MODERATE_CANOPY_DENSITY);

		let RiparianGeneralItem::HighBush(bush) = RiparianGeneralCell::RareRiparianHighBush.item()
		else {
			anyhow::bail!("expected high bush item");
		};
		assert_eq!(bush.height, UnitRange::new(5.0, 15.0));
		assert_eq!(bush.leaf_radius, UnitRange::new(0.12, 0.28));
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = RiparianGeneralCell::distribution();
		let braid_oak = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RiparianGeneralCell::RiparianBraidOak))
			.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
		assert_eq!(braid_oak.constraints.elevation.end, 0.42);
		assert_eq!(braid_oak.constraints.steepness.end, 0.36);

		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RiparianGeneralCell::RiparianStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.elevation.end, 0.45);
		assert_eq!(storybook.constraints.steepness.end, 0.44);

		let high_bush = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(RiparianGeneralCell::RareRiparianHighBush))
			.ok_or_else(|| anyhow::anyhow!("missing high bush bucket"))?;
		assert_eq!(high_bush.constraints.elevation.end, 0.38);
		assert_eq!(high_bush.constraints.steepness.end, 0.52);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_braid_oak_but_allows_high_bush() -> Result<()> {
		let prepared =
			RiparianGeneralCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.45 };
		let bush_outcome = prepared.select_from(5, Vec3::new(5.0, 0.25, 5.0), 1.0, &terrain);
		match bush_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, RiparianGeneralCell::RareRiparianHighBush);
			}
			other => anyhow::bail!("expected RareRiparianHighBush on moderate slope, got {other:?}"),
		}
		let braid_outcome = prepared.select_from(1, Vec3::new(5.0, 0.25, 5.0), 1.0, &terrain);
		match braid_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, RiparianGeneralCell::RiparianBraidOak);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			RiparianGeneralCell::RiparianBraidOak,
			RiparianGeneralCell::RiparianStorybook,
			RiparianGeneralCell::RareRiparianHighBush,
		] {
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0));
		let terrain = FlatTerrainSample { elevation: 0.20, steepness: 0.10 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
