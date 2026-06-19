//! Riverine Green — well-known sparse wet shrub understory grove
//! ([RFC-183 §3.4.5.10](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/10-riverine-green/README.md),
//! [#307](https://github.com/ramate-io/maybraid/issues/307)).
//!
//! Moderate-density Common High Bush punctuation along riparian edges. Each placement is a
//! single [`HighBushShoots`](../../tree-components/src/high_bush_shoots/assembly.rs) bush with
//! dual stick and canopy palettes; forest-layer attachment remains a follow-up.

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
pub use render::{RiverineGreen, RiverineGreenStd};

/// Authored Riverine Green grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`4.0..10.0`). The offset range
/// is signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<RiverineGreenCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(7.0),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-7.0, 7.0)),
		distribution: RiverineGreenCell::distribution(),
	}
}

/// Ordered riverine-green varietals ([RFC-183 §3.4.5.10]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiverineGreenCell {
	WetGreenBush,
	BrightBankBush,
	DeepShadeBush,
	PaleRiparianBush,
	RedTwigRiverBush,
}

/// Typed authored geometry for one riverine-green bush.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiverineGreenItem {
	Bush(&'static RiverineGreenBush),
}

/// Authored geometry ranges for one Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct RiverineGreenBush {
	pub height: UnitRange,
	pub shoot_count: RangeInclusive<u32>,
	pub branch_depth: RangeInclusive<u32>,
	/// RFC `projection_count` — horizontal splay in shoot direction mix.
	pub radial_strength: UnitRange,
	/// RFC `projection_count` — upward bias in shoot direction mix.
	pub vertical_bias: UnitRange,
	/// Terminal foliage radius in **world meters** (render converts to a height fraction).
	pub leaf_radius: UnitRange,
}

const WET_GREEN_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(1.00, 2.20),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: UnitRange::new(0.38, 0.52),
	vertical_bias: UnitRange::new(0.18, 0.82),
	leaf_radius: UnitRange::new(0.06, 0.13),
};

const BRIGHT_BANK_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(0.80, 1.70),
	shoot_count: 6..=10,
	branch_depth: 2..=3,
	radial_strength: UnitRange::new(0.42, 0.58),
	vertical_bias: UnitRange::new(0.22, 0.78),
	leaf_radius: UnitRange::new(0.05, 0.11),
};

const DEEP_SHADE_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(1.20, 2.40),
	shoot_count: 8..=12,
	branch_depth: 3..=5,
	radial_strength: UnitRange::new(0.30, 0.45),
	vertical_bias: UnitRange::new(0.72, 0.90),
	leaf_radius: UnitRange::new(0.07, 0.14),
};

const PALE_RIPARIAN_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(0.90, 1.80),
	shoot_count: 6..=10,
	branch_depth: 2..=4,
	radial_strength: UnitRange::new(0.35, 0.50),
	vertical_bias: UnitRange::new(0.18, 0.80),
	leaf_radius: UnitRange::new(0.05, 0.12),
};

const RED_TWIG_RIVER_BUSH: RiverineGreenBush = RiverineGreenBush {
	height: UnitRange::new(0.90, 1.90),
	shoot_count: 7..=11,
	branch_depth: 2..=4,
	radial_strength: UnitRange::new(0.38, 0.55),
	vertical_bias: UnitRange::new(0.18, 0.82),
	leaf_radius: UnitRange::new(0.05, 0.12),
};

const WET_GREEN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_bark", "dark_bark"),
	PaletteSlot::new("green_brown", "wet_brown"),
]);
const BRIGHT_BANK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("young_bark", "green_brown"),
	PaletteSlot::new("wet_brown", "tan_bark"),
]);
const DEEP_SHADE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_bark", "wet_brown"),
	PaletteSlot::new("green_brown", "gray_brown"),
]);
const PALE_RIPARIAN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_bark", "gray_brown"),
	PaletteSlot::new("green_brown", "tan_bark"),
]);
const RED_TWIG_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("red_twig", "copper_red"),
	PaletteSlot::new("wet_burgundy", "dark_bark"),
]);

const WET_GREEN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_green", "fresh_green"),
	PaletteSlot::new("deep_green", "light_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);
const BRIGHT_BANK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("bright_green", "light_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
	PaletteSlot::new("lush_green", "lime_green"),
]);
const DEEP_SHADE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("blue_green", "wet_green"),
	PaletteSlot::new("emerald_green", "fresh_green"),
]);
const PALE_RIPARIAN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("pale_green", "fresh_green"),
	PaletteSlot::new("silver_green", "light_green"),
	PaletteSlot::new("yellow_green", "wet_green"),
]);
const RED_TWIG_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_green", "fresh_green"),
	PaletteSlot::new("bright_green", "yellow_green"),
	PaletteSlot::new("silver_green", "light_green"),
]);

impl RiverineGreenCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.45` (RFC relative proportions); the `None` weight of `11.0` puts
	/// the placed share at `4.45 / 15.45 ≈ 0.29` — denser than the RFC midpoint while keeping
	/// shorelines readable.
	pub fn distribution() -> GroveDistribution<Self> {
		GroveDistribution::new(vec![
			GroveBucket::none(11.0),
			GroveBucket::placed(
				2.0,
				PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.42)),
				Self::WetGreenBush,
			),
			GroveBucket::placed(
				1.0,
				PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.65)),
				Self::BrightBankBush,
			),
			GroveBucket::placed(
				0.75,
				PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.45)),
				Self::DeepShadeBush,
			),
			GroveBucket::placed(
				0.45,
				PlacementConstraints::new(UnitRange::new(0.0, 0.42), UnitRange::new(0.0, 0.60)),
				Self::PaleRiparianBush,
			),
			GroveBucket::placed(
				0.25,
				PlacementConstraints::new(UnitRange::new(0.0, 0.38), UnitRange::new(0.0, 0.55)),
				Self::RedTwigRiverBush,
			),
		])
	}

	/// Authored geometry for this varietal.
	pub fn item(self) -> RiverineGreenItem {
		match self {
			Self::WetGreenBush => RiverineGreenItem::Bush(&WET_GREEN_BUSH),
			Self::BrightBankBush => RiverineGreenItem::Bush(&BRIGHT_BANK_BUSH),
			Self::DeepShadeBush => RiverineGreenItem::Bush(&DEEP_SHADE_BUSH),
			Self::PaleRiparianBush => RiverineGreenItem::Bush(&PALE_RIPARIAN_BUSH),
			Self::RedTwigRiverBush => RiverineGreenItem::Bush(&RED_TWIG_RIVER_BUSH),
		}
	}

	/// Authored stick palette for this varietal.
	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::WetGreenBush => WET_GREEN_STICK_MIX,
			Self::BrightBankBush => BRIGHT_BANK_STICK_MIX,
			Self::DeepShadeBush => DEEP_SHADE_STICK_MIX,
			Self::PaleRiparianBush => PALE_RIPARIAN_STICK_MIX,
			Self::RedTwigRiverBush => RED_TWIG_STICK_MIX,
		}
	}

	/// Authored canopy palette for this varietal.
	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::WetGreenBush => WET_GREEN_CANOPY_MIX,
			Self::BrightBankBush => BRIGHT_BANK_CANOPY_MIX,
			Self::DeepShadeBush => DEEP_SHADE_CANOPY_MIX,
			Self::PaleRiparianBush => PALE_RIPARIAN_CANOPY_MIX,
			Self::RedTwigRiverBush => RED_TWIG_CANOPY_MIX,
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
		let dist = RiverineGreenCell::distribution();
		assert_eq!(dist.len(), 6);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 11.0);
		assert_eq!(dist.buckets[1].item, Some(RiverineGreenCell::WetGreenBush));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(RiverineGreenCell::BrightBankBush));
		assert_eq!(dist.buckets[2].weight, 1.0);
		assert_eq!(dist.buckets[3].item, Some(RiverineGreenCell::DeepShadeBush));
		assert_eq!(dist.buckets[3].weight, 0.75);
		assert_eq!(dist.buckets[4].item, Some(RiverineGreenCell::PaleRiparianBush));
		assert_eq!(dist.buckets[4].weight, 0.45);
		assert_eq!(dist.buckets[5].item, Some(RiverineGreenCell::RedTwigRiverBush));
		assert_eq!(dist.buckets[5].weight, 0.25);
		Ok(())
	}

	#[test]
	fn placed_share_matches_moderate_riparian_target() -> Result<()> {
		let dist = RiverineGreenCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!(
			(0.25..=0.35).contains(&share),
			"placed share {share} outside moderate riparian band (~29 %)"
		);
		Ok(())
	}

	#[test]
	fn bush_geometry_follows_authored_bands() -> Result<()> {
		for cell in [
			RiverineGreenCell::WetGreenBush,
			RiverineGreenCell::BrightBankBush,
			RiverineGreenCell::DeepShadeBush,
			RiverineGreenCell::PaleRiparianBush,
			RiverineGreenCell::RedTwigRiverBush,
		] {
			let RiverineGreenItem::Bush(bush) = cell.item();
			assert!(bush.height.start >= 0.80);
			assert!(bush.height.end <= 2.40);
			assert!(*bush.shoot_count.start() >= 6);
			assert!(*bush.shoot_count.end() <= 12);
			assert!(bush.leaf_radius.start >= 0.05);
			assert!(bush.leaf_radius.end <= 0.14);
			assert!(bush.radial_strength.start >= 0.30);
			assert!(bush.radial_strength.end <= 0.58);
			assert!(bush.vertical_bias.start >= 0.18);
			assert!(bush.vertical_bias.end <= 0.90);
		}
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		// DeepShadeBush (index 3) rejects steepness 0.50; first-fit falls to PaleRiparianBush
		// (index 4), which allows steepness up to 0.60.
		let prepared =
			RiverineGreenCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.50 };
		let outcome = prepared.select_from(3, Vec3::new(5.0, 0.25, 5.0), 1.0, Cell::from_min_max(Vec3::ZERO, Vec3::ONE), &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, RiverineGreenCell::PaleRiparianBush);
			}
			other => anyhow::bail!("expected PaleRiparianBush fallback, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.15 };
		let placements = grove.populate(&extent, &terrain);
		assert!(!placements.is_empty());

		let cell = definition().cell_extent_xz.x;
		let off_center = placements
			.iter()
			.filter(|p| {
				let local_x = (p.position.x / cell).fract() - 0.5;
				let local_z = (p.position.z / cell).fract() - 0.5;
				local_x.abs() > 0.25 || local_z.abs() > 0.25
			})
			.count();
		assert!(
			off_center * 2 >= placements.len(),
			"expected at least half of {} placements off cell centers, got {off_center}",
			placements.len()
		);
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
