//! Braid Grass — well-known understory grove ([RFC-183 §3.4.5.1], [#306](https://github.com/ramate-io/maybraid/issues/306)).
//!
//! All authored data (cell footprint, placement ranges, bucket weights, constraints, palettes,
//! and clump geometry) lives in this module as constants mirroring the RFC blocks.

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
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-3.0, 3.0),
		),
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
}

/// Authored geometry ranges for one braid-grass clump.
#[derive(Debug, Clone, PartialEq)]
pub struct BraidGrassClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**; absolute widths render far-too-thick
	/// blades (the RFC widths describe the clump footprint, not blade thickness).
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub braid_twist: UnitRange,
}

/// Shared blade thickness band: ~2–3 % of blade length — braid blades run long (1–3 m),
/// so the proportional band is tighter than the short-tuft groves.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.03);

const DEEP_GREEN_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.0, 2.2),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 12..=28,
	braid_twist: UnitRange::new(0.10, 0.35),
};

const PALE_REED_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.2, 2.6),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=22,
	braid_twist: UnitRange::new(0.05, 0.25),
};

const JUNGLE_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.6, 3.0),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 18..=36,
	braid_twist: UnitRange::new(0.20, 0.50),
};

const RED_EDGE_BLADE: BraidGrassClump = BraidGrassClump {
	height: UnitRange::new(1.0, 2.0),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=24,
	braid_twist: UnitRange::new(0.10, 0.30),
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
		])
	}

	/// Authored geometry ranges for this variant.
	pub fn clump(self) -> &'static BraidGrassClump {
		match self {
			Self::DeepGreenBlade => &DEEP_GREEN_BLADE,
			Self::PaleReedBlade => &PALE_REED_BLADE,
			Self::JungleBlade => &JUNGLE_BLADE,
			Self::RedEdgeBlade => &RED_EDGE_BLADE,
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
		match self {
			Self::DeepGreenBlade => DEEP_GREEN_MIX,
			Self::PaleReedBlade => PALE_REED_MIX,
			Self::JungleBlade => JUNGLE_MIX,
			Self::RedEdgeBlade => RED_EDGE_MIX,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{
		ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent, FlatTerrainSample,
	};
	use anyhow::Result;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = BraidGrassCell::distribution();
		assert_eq!(dist.len(), 5);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 2.5);
		assert_eq!(dist.buckets[1].item, Some(BraidGrassCell::DeepGreenBlade));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].weight, 1.0);
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(BraidGrassCell::RedEdgeBlade));
		assert_eq!(dist.buckets[4].weight, 0.5);
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
