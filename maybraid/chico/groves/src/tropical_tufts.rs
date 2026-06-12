//! Tropical Tufts — well-known sparse tuft grove with palm companions
//! ([RFC-183 §3.4.4.5], [#305](https://github.com/ramate-io/maybraid/issues/305)).
//!
//! All authored data (cell footprint, placement ranges, bucket weights, constraints, palettes,
//! and item geometry) lives in this module as constants mirroring the RFC blocks.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix,
	PaletteSlot, PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{TropicalTufts, TropicalTuftsStd};

/// Authored Tropical Tufts grove definition.
///
/// The offset range is signed and wider than the RFC's nominal `0.0..1.0` (± one cell) so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<TropicalTuftsCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(3.25),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-3.25, 3.25),
		),
		distribution: TropicalTuftsCell::distribution(),
	}
}

/// Ordered tropical-tufts variants ([RFC-183 §3.4.2.2]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalTuftsCell {
	BrightTuft,
	DeepTuft,
	YellowGreenTuft,
	SmallPalmBush,
	JuvenilePalmBush,
}

/// Typed authored geometry for one tropical-tufts variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TropicalTuftsItem {
	Tuft(&'static TropicalTuftClump),
	PalmBush(&'static TropicalPalmBush),
}

/// Authored geometry ranges for one tropical tuft clump.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalTuftClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**; absolute widths render far-too-thick
	/// blades (the RFC widths describe the clump footprint, not blade thickness).
	pub width_factor: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

/// Authored geometry ranges for one ground-anchored palm bush companion.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalPalmBush {
	pub height: UnitRange,
	pub frond_count: RangeInclusive<u32>,
	pub frond_length: UnitRange,
	pub crown_spread: UnitRange,
}

const BRIGHT_TUFT: TropicalTuftClump = TropicalTuftClump {
	height: UnitRange::new(0.25, 0.50),
	width_factor: BLADE_WIDTH_FACTOR,
};

const DEEP_TUFT: TropicalTuftClump = TropicalTuftClump {
	height: UnitRange::new(0.30, 0.55),
	width_factor: BLADE_WIDTH_FACTOR,
};

const YELLOW_GREEN_TUFT: TropicalTuftClump = TropicalTuftClump {
	height: UnitRange::new(0.25, 0.45),
	width_factor: BLADE_WIDTH_FACTOR,
};

const SMALL_PALM_BUSH: TropicalPalmBush = TropicalPalmBush {
	height: UnitRange::new(0.35, 0.80),
	frond_count: 4..=7,
	frond_length: UnitRange::new(0.18, 0.45),
	crown_spread: UnitRange::new(0.25, 0.55),
};

const JUVENILE_PALM_BUSH: TropicalPalmBush = TropicalPalmBush {
	height: UnitRange::new(0.50, 1.10),
	frond_count: 3..=5,
	frond_length: UnitRange::new(0.25, 0.60),
	crown_spread: UnitRange::new(0.30, 0.70),
};

impl TropicalTuftsCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	pub fn distribution() -> GroveDistribution<Self> {
		let bright =
			PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.35));
		let lowland =
			PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.75));
		let juvenile =
			PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.75));
		GroveDistribution::new(vec![
			GroveBucket::none(29.4),
			GroveBucket::placed(2.0, bright, Self::BrightTuft),
			GroveBucket::placed(1.5, lowland, Self::DeepTuft),
			GroveBucket::placed(1.0, lowland, Self::YellowGreenTuft),
			GroveBucket::placed(0.75, lowland, Self::SmallPalmBush),
			GroveBucket::placed(0.35, juvenile, Self::JuvenilePalmBush),
		])
	}

	/// Authored geometry for this variant.
	pub fn item(self) -> TropicalTuftsItem {
		match self {
			Self::BrightTuft => TropicalTuftsItem::Tuft(&BRIGHT_TUFT),
			Self::DeepTuft => TropicalTuftsItem::Tuft(&DEEP_TUFT),
			Self::YellowGreenTuft => TropicalTuftsItem::Tuft(&YELLOW_GREEN_TUFT),
			Self::SmallPalmBush => TropicalTuftsItem::PalmBush(&SMALL_PALM_BUSH),
			Self::JuvenilePalmBush => TropicalTuftsItem::PalmBush(&JUVENILE_PALM_BUSH),
		}
	}

	/// Authored palette ranges for this variant.
	pub fn palette_mix(self) -> PaletteMix {
		const BRIGHT_TUFT_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("bright_green", "lime_green"),
			PaletteSlot::new("lush_green", "fresh_green"),
			PaletteSlot::new("yellow_green", "light_green"),
		]);
		const DEEP_TUFT_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("deep_green", "emerald_green"),
			PaletteSlot::new("dark_green", "wet_green"),
			PaletteSlot::new("blue_green", "bright_green"),
		]);
		const YELLOW_GREEN_TUFT_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("yellow_green", "fresh_green"),
			PaletteSlot::new("lime_green", "light_green"),
			PaletteSlot::new("young_green", "bright_green"),
		]);
		const SMALL_PALM_BUSH_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("lush_green", "bright_green"),
			PaletteSlot::new("deep_green", "fresh_green"),
			PaletteSlot::new("wet_green", "lime_green"),
		]);
		const JUVENILE_PALM_BUSH_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("young_green", "lime_green"),
			PaletteSlot::new("fresh_green", "light_green"),
			PaletteSlot::new("bright_green", "yellow_green"),
		]);
		match self {
			Self::BrightTuft => BRIGHT_TUFT_MIX,
			Self::DeepTuft => DEEP_TUFT_MIX,
			Self::YellowGreenTuft => YELLOW_GREEN_TUFT_MIX,
			Self::SmallPalmBush => SMALL_PALM_BUSH_MIX,
			Self::JuvenilePalmBush => JUVENILE_PALM_BUSH_MIX,
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
		let dist = TropicalTuftsCell::distribution();
		assert_eq!(dist.len(), 6);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 29.4);
		assert_eq!(dist.buckets[1].item, Some(TropicalTuftsCell::BrightTuft));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].weight, 1.5);
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(TropicalTuftsCell::SmallPalmBush));
		assert_eq!(dist.buckets[4].weight, 0.75);
		assert_eq!(dist.buckets[5].item, Some(TropicalTuftsCell::JuvenilePalmBush));
		assert_eq!(dist.buckets[5].weight, 0.35);
		Ok(())
	}

	#[test]
	fn variants_map_to_typed_items() -> Result<()> {
		assert!(matches!(TropicalTuftsCell::BrightTuft.item(), TropicalTuftsItem::Tuft(_)));
		let TropicalTuftsItem::PalmBush(palm) = TropicalTuftsCell::SmallPalmBush.item() else {
			anyhow::bail!("expected palm bush item");
		};
		assert_eq!(palm.frond_count, 4..=7);
		Ok(())
	}

	#[test]
	fn first_fit_from_placed_bucket_places_variant() -> Result<()> {
		let prepared = TropicalTuftsCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.4, steepness: 0.1 };
		let outcome = prepared.select_from(1, Vec3::new(5.0, 0.4, 5.0), 1.0, &terrain);
		assert!(matches!(outcome, GroveCellOutcome::Placed { .. }));
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic() -> Result<()> {
		let grove = Grove::assemble(
			definition(),
			ForestGroveBiases::default(),
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(20.0, 1.0, 20.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		assert_eq!(grove.populate(&extent, &terrain), grove.populate(&extent, &terrain));
		Ok(())
	}
}
