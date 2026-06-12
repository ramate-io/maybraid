//! Braid Grass — well-known understory grove ([RFC-183 §3.4.5.1], [#306](https://github.com/ramate-io/maybraid/issues/306)).
//!
//! All authored data (cell footprint, placement ranges, bucket weights, constraints, palettes,
//! and clump geometry) lives in this module as constants mirroring the RFC blocks.

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
pub use render::{BraidGrass, BraidGrassStd};

/// Authored Braid Grass grove definition.
///
/// The cell footprint is denser than the RFC's nominal grid to keep preview groves visually
/// populated; forest gridding may override it per grove. The offset range is wider than the
/// RFC's nominal ±1 m so biased sampling plus noise still reaches meaningful horizontal
/// variety; [`crate::grove::GroveExtent`] validation keeps the grove LOD unit bounded.
pub fn definition() -> GroveDefinition<BraidGrassCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.125),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-3.0, 3.0)),
		distribution: BraidGrassCell::distribution(),
	}
}

/// Ordered braid-grass variants ([RFC-183 §3.4.2.2]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BraidGrassCell {
	DeepGreenBlade,
	PaleReedBlade,
	JungleBlade,
	RedEdgeBlade,
	GreenSpear,
	FountainSpear,
}

/// Typed authored geometry for one braid-grass variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BraidGrassItem {
	Blade(&'static BraidGrassClump),
	Spear(&'static BraidSpearClump),
}

/// Authored geometry ranges for one braid-grass blade clump.
#[derive(Debug, Clone, PartialEq)]
pub struct BraidGrassClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**; absolute widths render far-too-thick
	/// blades (the RFC widths describe the clump footprint, not blade thickness).
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	/// Max polar blade tilt — the RFC's "braid twist".
	pub max_tilt_radians: UnitRange,
}

/// Authored geometry ranges for one braid-grass spear clump (flat belly→tip ribbons).
#[derive(Debug, Clone, PartialEq)]
pub struct BraidSpearClump {
	pub height: UnitRange,
	/// Belly half-width as a **fraction of spear length** (same proportional contract as
	/// [`BraidGrassClump::width_factor`]); the base tapers from the belly in the render build.
	pub belly_factor: UnitRange,
	pub spear_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2–3 % of blade length — braid blades run long (1–3 m),
/// so the proportional band is tighter than the short-tuft groves.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.022, 0.034);

/// Braid Grass takes the widest shape variation of the tuft groves: kink counts span
/// near-straight reeds through heavily braided blades.
const BEND_SEGMENTS: RangeInclusive<u32> = 2..=10;

// Most tilt bands sit in a moderate regime (uprightish PaleReed/GreenSpear through leaning
// DeepGreen/RedEdge); Jungle and FountainSpear instead take *wide* bands, so individual
// clumps of those varietals range anywhere from upright to fully splayed.

const DEEP_GREEN_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.0, 2.2),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 8..=28,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.20, 0.45),
};

const PALE_REED_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.2, 2.6),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 6..=22,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.10, 0.25),
};

const JUNGLE_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.6, 3.0),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 6..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.15, 0.90),
};

const RED_EDGE_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.0, 2.0),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=18,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.25, 0.55),
};

const GREEN_SPEAR: BraidSpearClump = BraidSpearClump {
	height: UnitRange::new(1.2, 2.4),
	belly_factor: UnitRange::new(0.008, 0.015),
	spear_count: 10..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.12, 0.30),
};

const FOUNTAIN_SPEAR: BraidSpearClump = BraidSpearClump {
	height: UnitRange::new(1.0, 2.0),
	belly_factor: UnitRange::new(0.010, 0.018),
	spear_count: 14..=30,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.20, 1.10),
};

impl BraidGrassCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	pub fn distribution() -> GroveDistribution<Self> {
		let low_ground =
			PlacementConstraints::new(UnitRange::new(0.0, 0.75), UnitRange::new(0.0, 0.60));
		let jungle_floor =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.30));
		let red_edge_ground =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(2.5),
			GroveBucket::placed(2.0, low_ground, Self::DeepGreenBlade),
			GroveBucket::placed(1.0, low_ground, Self::PaleReedBlade),
			GroveBucket::placed(1.0, jungle_floor, Self::JungleBlade),
			GroveBucket::placed(0.5, red_edge_ground, Self::RedEdgeBlade),
			GroveBucket::placed(1.0, low_ground, Self::GreenSpear),
			GroveBucket::placed(0.75, jungle_floor, Self::FountainSpear),
		])
	}

	/// Authored geometry for this variant.
	pub fn item(self) -> BraidGrassItem {
		match self {
			Self::DeepGreenBlade => BraidGrassItem::Blade(&DEEP_GREEN_BLADE),
			Self::PaleReedBlade => BraidGrassItem::Blade(&PALE_REED_BLADE),
			Self::JungleBlade => BraidGrassItem::Blade(&JUNGLE_BLADE),
			Self::RedEdgeBlade => BraidGrassItem::Blade(&RED_EDGE_BLADE),
			Self::GreenSpear => BraidGrassItem::Spear(&GREEN_SPEAR),
			Self::FountainSpear => BraidGrassItem::Spear(&FOUNTAIN_SPEAR),
		}
	}

	/// Authored palette ranges for this variant.
	pub fn palette_mix(self) -> PaletteMix {
		const DEEP_GREEN_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("deep_green", "wet_green"),
			PaletteSlot::new("dark_green", "emerald_green"),
			PaletteSlot::new("blue_green", "fresh_green"),
		]);
		const PALE_REED_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("yellow_green", "pale_straw"),
			PaletteSlot::new("dry_green", "light_green"),
			PaletteSlot::new("tan_green", "fresh_green"),
		]);
		const JUNGLE_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("lush_green", "bright_green"),
			PaletteSlot::new("wet_green", "lime_green"),
			PaletteSlot::new("blue_green", "deep_green"),
		]);
		const RED_EDGE_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("red_green", "deep_green"),
			PaletteSlot::new("copper_red", "yellow_green"),
			PaletteSlot::new("dark_red", "wet_green"),
		]);
		const GREEN_SPEAR_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("emerald_green", "fresh_green"),
			PaletteSlot::new("deep_green", "lime_green"),
			PaletteSlot::new("wet_green", "bright_green"),
		]);
		const FOUNTAIN_SPEAR_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("bright_green", "lime_green"),
			PaletteSlot::new("yellow_green", "fresh_green"),
			PaletteSlot::new("lush_green", "light_green"),
		]);
		match self {
			Self::DeepGreenBlade => DEEP_GREEN_MIX,
			Self::PaleReedBlade => PALE_REED_MIX,
			Self::JungleBlade => JUNGLE_MIX,
			Self::RedEdgeBlade => RED_EDGE_MIX,
			Self::GreenSpear => GREEN_SPEAR_MIX,
			Self::FountainSpear => FOUNTAIN_SPEAR_MIX,
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
		let dist = BraidGrassCell::distribution();
		assert_eq!(dist.len(), 7);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 2.5);
		assert_eq!(dist.buckets[1].item, Some(BraidGrassCell::DeepGreenBlade));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].weight, 1.0);
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(BraidGrassCell::RedEdgeBlade));
		assert_eq!(dist.buckets[4].weight, 0.5);
		assert_eq!(dist.buckets[5].item, Some(BraidGrassCell::GreenSpear));
		assert_eq!(dist.buckets[5].weight, 1.0);
		assert_eq!(dist.buckets[6].item, Some(BraidGrassCell::FountainSpear));
		assert_eq!(dist.buckets[6].weight, 0.75);
		Ok(())
	}

	#[test]
	fn tilt_bands_mix_moderate_and_wide() -> Result<()> {
		// Most varietals stay in a moderate tilt regime; Jungle and FountainSpear take wide
		// bands so their individual clumps span upright through fully splayed.
		let tilt = |cell: BraidGrassCell| match cell.item() {
			BraidGrassItem::Blade(clump) => clump.max_tilt_radians,
			BraidGrassItem::Spear(clump) => clump.max_tilt_radians,
		};
		for moderate in [
			BraidGrassCell::DeepGreenBlade,
			BraidGrassCell::PaleReedBlade,
			BraidGrassCell::RedEdgeBlade,
			BraidGrassCell::GreenSpear,
		] {
			let band = tilt(moderate);
			assert!(band.start >= 0.05, "{moderate:?} should not be extreme-upright");
			assert!(band.end <= 0.60, "{moderate:?} should not be extreme-splayed");
		}
		for wide in [BraidGrassCell::JungleBlade, BraidGrassCell::FountainSpear] {
			let band = tilt(wide);
			assert!(band.end - band.start >= 0.6, "{wide:?} should span a wide tilt band");
		}
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		// Jungle (index 3) rejects steepness 0.35; first-fit wraps to RedEdge (index 4).
		let prepared =
			BraidGrassCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.3, steepness: 0.35 };
		let outcome = prepared.select_from(3, Vec3::new(5.0, 0.3, 5.0), 1.0, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, BraidGrassCell::RedEdgeBlade);
			}
			other => anyhow::bail!("expected RedEdgeBlade fallback, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let grove = Grove::assemble(
			definition(),
			ForestGroveBiases::default(),
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
