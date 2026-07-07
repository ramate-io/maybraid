//! Forlorn Savanna — low-density sparse dry upper-canopy grove
//! ([RFC-183 §3.4.7.6], [#351](https://github.com/ramate-io/maybraid/issues/351)).
//!
//! Wind-shaped Rory's Head-trained forms, acacia-impression High Bush, and rare dry Storybook
//! accents across open savanna. Forest-layer attachment remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);
/// Flat sparse crown projection for acacia-like High Bush forms.
const SPARSE_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.42, 0.62);
const SPARSE_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.32, 0.52);

/// Authored Forlorn Savanna grove definition.
///
/// Cell footprint sits at the RFC midpoint (`30` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<ForlornSavannaCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(30.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-30.0, 30.0),
		),
		distribution: ForlornSavannaCell::distribution(),
	}
}

/// Ordered forlorn-savanna varietals ([RFC-183 §3.4.7.6]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForlornSavannaCell {
	SavannaRory,
	AcaciaHighBush,
	RareSavannaStorybook,
}

/// Typed authored geometry for one forlorn-savanna varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForlornSavannaItem {
	Rory(&'static ForlornSavannaRory),
	HighBush(&'static ForlornSavannaHighBush),
	Storybook(&'static ForlornSavannaStorybook),
}

/// Authored geometry ranges for one Rory's Head-trained form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaRory {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one acacia-impression Common High Bush form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaHighBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	pub radial_strength: UnitRange,
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

/// Authored geometry ranges for one dry Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct ForlornSavannaStorybook {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const SAVANNA_RORY: ForlornSavannaRory = ForlornSavannaRory {
	height: UnitRange::new(5.0, 30.0),
	stalk_radius: UnitRange::new(0.12, 0.45),
	canopy_spread: UnitRange::new(3.0, 12.0),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const ACACIA_HIGH_BUSH: ForlornSavannaHighBush = ForlornSavannaHighBush {
	height: UnitRange::new(5.0, 10.0),
	shoot_count: 4..=12,
	branch_depth: 2..=3,
	radial_strength: SPARSE_PROJECTION_RADIAL,
	vertical_bias: SPARSE_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.35, 0.55),
};

const RARE_SAVANNA_STORYBOOK: ForlornSavannaStorybook = ForlornSavannaStorybook {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.24, 0.52),
	canopy_spread: UnitRange::new(2.5, 6.5),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const SAVANNA_RORY_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("weathered_bark", "dark_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const SAVANNA_RORY_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dry_green"),
	PaletteSlot::new("yellow_green", "dusty_green"),
]);

const ACACIA_HIGH_BUSH_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("acacia_bark", "red_brown"),
	PaletteSlot::new("tan_bark", "gray_brown"),
]);

const ACACIA_HIGH_BUSH_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dusty_green", "olive_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

const SAVANNA_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_brown", "dark_bark"),
	PaletteSlot::new("gray_brown", "tan_bark"),
]);

const SAVANNA_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "yellow_green"),
	PaletteSlot::new("dusty_green", "light_green"),
]);

impl ForlornSavannaCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.2`; the `None` weight of `30.0` puts the placed share at
	/// `5.2 / 35.2 ≈ 0.15`, mid RFC `DENSITY_RANGE` (`0.06..0.20`).
	pub fn distribution() -> GroveDistribution<Self> {
		let rory = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.58));
		let high_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.64));
		let storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.50));
		GroveDistribution::new(vec![
			GroveBucket::none(30.0),
			GroveBucket::placed(3.0, rory, Self::SavannaRory),
			GroveBucket::placed(2.0, high_bush, Self::AcaciaHighBush),
			GroveBucket::placed(0.2, storybook, Self::RareSavannaStorybook),
		])
	}

	pub fn item(self) -> ForlornSavannaItem {
		match self {
			Self::SavannaRory => ForlornSavannaItem::Rory(&SAVANNA_RORY),
			Self::AcaciaHighBush => ForlornSavannaItem::HighBush(&ACACIA_HIGH_BUSH),
			Self::RareSavannaStorybook => ForlornSavannaItem::Storybook(&RARE_SAVANNA_STORYBOOK),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SavannaRory => SAVANNA_RORY_STICK_MIX,
			Self::AcaciaHighBush => ACACIA_HIGH_BUSH_STICK_MIX,
			Self::RareSavannaStorybook => SAVANNA_STORYBOOK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SavannaRory => SAVANNA_RORY_CANOPY_MIX,
			Self::AcaciaHighBush => ACACIA_HIGH_BUSH_CANOPY_MIX,
			Self::RareSavannaStorybook => SAVANNA_STORYBOOK_CANOPY_MIX,
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
		let dist = ForlornSavannaCell::distribution();
		assert_eq!(dist.len(), 4);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 30.0);
		assert_eq!(dist.buckets[1].item, Some(ForlornSavannaCell::SavannaRory));
		assert_eq!(dist.buckets[1].weight, 3.0);
		assert_eq!(dist.buckets[2].item, Some(ForlornSavannaCell::AcaciaHighBush));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(ForlornSavannaCell::RareSavannaStorybook));
		assert_eq!(dist.buckets[3].weight, 0.2);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = ForlornSavannaCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.06..=0.20).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let ForlornSavannaItem::Rory(rory) = ForlornSavannaCell::SavannaRory.item() else {
			anyhow::bail!("expected rory item");
		};
		assert_eq!(rory.height, UnitRange::new(5.0, 30.0));
		assert_eq!(rory.canopy_spread, UnitRange::new(3.0, 12.0));
		assert_eq!(rory.canopy_density, SPARSE_CANOPY_DENSITY);

		let ForlornSavannaItem::HighBush(bush) = ForlornSavannaCell::AcaciaHighBush.item() else {
			anyhow::bail!("expected high bush item");
		};
		assert_eq!(bush.height, UnitRange::new(5.0, 10.0));

		let ForlornSavannaItem::Storybook(story) = ForlornSavannaCell::RareSavannaStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		assert_eq!(story.height, UnitRange::new(10.0, 20.0));
		assert_eq!(story.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = ForlornSavannaCell::distribution();
		let rory = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ForlornSavannaCell::SavannaRory))
			.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
		assert_eq!(rory.constraints.elevation.start, 0.0);
		assert_eq!(rory.constraints.elevation.end, 1.0);
		assert_eq!(rory.constraints.steepness.end, 0.58);

		let high_bush = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ForlornSavannaCell::AcaciaHighBush))
			.ok_or_else(|| anyhow::anyhow!("missing high bush bucket"))?;
		assert_eq!(high_bush.constraints.elevation.end, 1.0);
		assert_eq!(high_bush.constraints.steepness.end, 0.64);

		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(ForlornSavannaCell::RareSavannaStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.elevation.start, 0.0);
		assert_eq!(storybook.constraints.steepness.end, 0.50);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_rory_but_allows_high_bush() -> Result<()> {
		let prepared = ForlornSavannaCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.60 };
		let bush_outcome = prepared.select_from(
			5,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match bush_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ForlornSavannaCell::AcaciaHighBush);
			}
			other => anyhow::bail!("expected AcaciaHighBush on moderate slope, got {other:?}"),
		}
		let rory_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match rory_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, ForlornSavannaCell::SavannaRory);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			ForlornSavannaCell::SavannaRory,
			ForlornSavannaCell::AcaciaHighBush,
			ForlornSavannaCell::RareSavannaStorybook,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.20 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
