//! Monster Grass — well-known oversized understory blade grove
//! ([RFC-183 §3.4.5.2](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/02-monster-grass/README.md),
//! [#308](https://github.com/ramate-io/maybraid/issues/308)).
//!
//! Dense 2–6 m understory blades for jungle, swamp, and elder-tree floors — structurally
//! Braid Grass at monster scale. RFC `droop` maps to `max_tilt_radians` on upward-biased blade
//! tufts; true downward sag remains a follow-up.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Authored Monster Grass grove definition.
///
/// Cell footprint is denser than the RFC's nominal `4.0..9.0` grid (like Braid Grass) so preview
/// groves read as continuous tall understory rather than sparse screens. The offset range is
/// signed and ± one cell so placements break the underlying grid instead of clustering near
/// cell centers.
pub fn definition() -> GroveDefinition<MonsterGrassCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-2.5, 2.5)),
		distribution: MonsterGrassCell::distribution(),
	}
}

/// Ordered monster-grass varietals ([RFC-183 §3.4.5.2]); the explicit `None` bucket lives only
/// in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterGrassCell {
	GiantWetBlade,
	BroadJungleBlade,
	PaleGiantReed,
	RedRibbedBlade,
	GiantWetBladePatch,
	BroadJungleBladePatch,
	PaleGiantReedPatch,
	RedRibbedBladePatch,
}

/// Typed authored geometry for one monster-grass varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MonsterGrassItem {
	Clump(&'static MonsterGrassClump),
	Patch(&'static GroveTuftPatch<MonsterGrassClump>),
}

/// Authored geometry ranges for one monster-grass blade clump.
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterGrassClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**. The RFC's absolute widths describe the
	/// clump footprint, not blade thickness.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	/// RFC `droop` — splay/sag departure from vertical on upward-biased blade tufts.
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2.5–4.5 % of blade length — broader than Braid Grass for the
/// heavy, wall-like read at 2–6 m.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.025, 0.045);

const BEND_SEGMENTS: RangeInclusive<u32> = 4..=12;

const GIANT_WET_BLADE: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.00, 6.00),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=28,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.25, 0.70),
};

const BROAD_JUNGLE_BLADE: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.50, 5.00),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 8..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.35, 0.85),
};

const PALE_GIANT_REED: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.00, 4.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 8..=22,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.15, 0.50),
};

const RED_RIBBED_BLADE: MonsterGrassClump = MonsterGrassClump {
	height: UnitRange::new(2.20, 4.20),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: 10..=24,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: UnitRange::new(0.20, 0.65),
};

// Patch varietals scatter tall blade clumps as loose mounds — Braid Grass geometry scaled up,
// with enough clumps per patch to read as dense understory rather than isolated walls.

const GIANT_WET_BLADE_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: GIANT_WET_BLADE,
	clump_count: 3..=5,
	patch_extent_xz: UnitRange::new(1.8, 4.4),
	base_spread: UnitRange::new(0.25, 0.50),
};

const BROAD_JUNGLE_BLADE_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: BROAD_JUNGLE_BLADE,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.6, 4.8),
	base_spread: UnitRange::new(0.30, 0.55),
};

const PALE_GIANT_REED_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: PALE_GIANT_REED,
	clump_count: 3..=5,
	patch_extent_xz: UnitRange::new(2.0, 2.8),
	base_spread: UnitRange::new(0.20, 0.45),
};

const RED_RIBBED_BLADE_PATCH: GroveTuftPatch<MonsterGrassClump> = GroveTuftPatch {
	clump: RED_RIBBED_BLADE,
	clump_count: 2..=5,
	patch_extent_xz: UnitRange::new(1.8, 4.4),
	base_spread: UnitRange::new(0.25, 0.50),
};

impl MonsterGrassCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.6` (RFC relative proportions); the `None` weight of `1.5` puts
	/// the placed share at `4.6 / 6.1 ≈ 0.75`, matching the dense understory read of Braid
	/// Grass. Patches carry `3.68` of the placed weight; single-anchor clumps share the
	/// remaining `0.92`.
	pub fn distribution() -> GroveDistribution<Self> {
		let low_wet =
			PlacementConstraints::new(UnitRange::new(0.0, 0.75), UnitRange::new(0.0, 0.50));
		let red_ribbed =
			PlacementConstraints::new(UnitRange::new(0.0, 0.75), UnitRange::new(0.0, 0.60));
		GroveDistribution::new(vec![
			GroveBucket::none(1.5),
			GroveBucket::placed(0.40, low_wet, Self::GiantWetBlade),
			GroveBucket::placed(0.30, low_wet, Self::BroadJungleBlade),
			GroveBucket::placed(0.15, low_wet, Self::PaleGiantReed),
			GroveBucket::placed(0.07, red_ribbed, Self::RedRibbedBlade),
			GroveBucket::placed(1.60, low_wet, Self::GiantWetBladePatch),
			GroveBucket::placed(1.20, low_wet, Self::BroadJungleBladePatch),
			GroveBucket::placed(0.60, low_wet, Self::PaleGiantReedPatch),
			GroveBucket::placed(0.28, red_ribbed, Self::RedRibbedBladePatch),
		])
	}

	/// Authored geometry for this varietal.
	pub fn item(self) -> MonsterGrassItem {
		match self {
			Self::GiantWetBlade => MonsterGrassItem::Clump(&GIANT_WET_BLADE),
			Self::BroadJungleBlade => MonsterGrassItem::Clump(&BROAD_JUNGLE_BLADE),
			Self::PaleGiantReed => MonsterGrassItem::Clump(&PALE_GIANT_REED),
			Self::RedRibbedBlade => MonsterGrassItem::Clump(&RED_RIBBED_BLADE),
			Self::GiantWetBladePatch => MonsterGrassItem::Patch(&GIANT_WET_BLADE_PATCH),
			Self::BroadJungleBladePatch => MonsterGrassItem::Patch(&BROAD_JUNGLE_BLADE_PATCH),
			Self::PaleGiantReedPatch => MonsterGrassItem::Patch(&PALE_GIANT_REED_PATCH),
			Self::RedRibbedBladePatch => MonsterGrassItem::Patch(&RED_RIBBED_BLADE_PATCH),
		}
	}

	/// Authored palette ranges for this varietal.
	pub fn palette_mix(self) -> PaletteMix {
		const GIANT_WET_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("deep_green", "wet_green"),
			PaletteSlot::new("blue_green", "dark_green"),
			PaletteSlot::new("emerald_green", "fresh_green"),
		]);
		const BROAD_JUNGLE_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("lush_green", "bright_green"),
			PaletteSlot::new("wet_green", "lime_green"),
			PaletteSlot::new("dark_green", "blue_green"),
		]);
		const PALE_GIANT_REED_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("yellow_green", "pale_straw"),
			PaletteSlot::new("dry_green", "tan_green"),
			PaletteSlot::new("light_green", "fresh_green"),
		]);
		const RED_RIBBED_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("dark_red", "deep_green"),
			PaletteSlot::new("copper_red", "wet_green"),
			PaletteSlot::new("red_green", "blue_green"),
		]);
		match self {
			Self::GiantWetBlade | Self::GiantWetBladePatch => GIANT_WET_MIX,
			Self::BroadJungleBlade | Self::BroadJungleBladePatch => BROAD_JUNGLE_MIX,
			Self::PaleGiantReed | Self::PaleGiantReedPatch => PALE_GIANT_REED_MIX,
			Self::RedRibbedBlade | Self::RedRibbedBladePatch => RED_RIBBED_MIX,
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
		let dist = MonsterGrassCell::distribution();
		assert_eq!(dist.len(), 9);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 1.5);
		assert_eq!(dist.buckets[1].item, Some(MonsterGrassCell::GiantWetBlade));
		assert_eq!(dist.buckets[1].weight, 0.40);
		assert_eq!(dist.buckets[2].item, Some(MonsterGrassCell::BroadJungleBlade));
		assert_eq!(dist.buckets[2].weight, 0.30);
		assert_eq!(dist.buckets[3].item, Some(MonsterGrassCell::PaleGiantReed));
		assert_eq!(dist.buckets[3].weight, 0.15);
		assert_eq!(dist.buckets[4].item, Some(MonsterGrassCell::RedRibbedBlade));
		assert_eq!(dist.buckets[4].weight, 0.07);
		assert_eq!(dist.buckets[5].item, Some(MonsterGrassCell::GiantWetBladePatch));
		assert_eq!(dist.buckets[5].weight, 1.60);
		assert_eq!(dist.buckets[6].item, Some(MonsterGrassCell::BroadJungleBladePatch));
		assert_eq!(dist.buckets[6].weight, 1.20);
		assert_eq!(dist.buckets[7].item, Some(MonsterGrassCell::PaleGiantReedPatch));
		assert_eq!(dist.buckets[7].weight, 0.60);
		assert_eq!(dist.buckets[8].item, Some(MonsterGrassCell::RedRibbedBladePatch));
		assert_eq!(dist.buckets[8].weight, 0.28);
		Ok(())
	}

	#[test]
	fn placed_share_matches_dense_understory_target() -> Result<()> {
		let dist = MonsterGrassCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!(
			(0.70..=0.80).contains(&share),
			"placed share {share} outside dense understory band (~75 %)"
		);
		Ok(())
	}

	#[test]
	fn patches_outweigh_single_clumps() -> Result<()> {
		let placed_weight = |patch: bool| -> f32 {
			MonsterGrassCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| {
						matches!(cell.item(), MonsterGrassItem::Patch(_)) == patch
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		assert!(
			placed_weight(true) > 2.0 * placed_weight(false),
			"patches should dominate placed weight"
		);
		Ok(())
	}

	#[test]
	fn clump_geometry_follows_authored_bands() -> Result<()> {
		for cell in [
			MonsterGrassCell::GiantWetBlade,
			MonsterGrassCell::BroadJungleBlade,
			MonsterGrassCell::PaleGiantReed,
			MonsterGrassCell::RedRibbedBlade,
		] {
			let MonsterGrassItem::Clump(clump) = cell.item() else {
				anyhow::bail!("expected clump item for {cell:?}");
			};
			assert!(clump.height.start >= 2.0);
			assert!(clump.height.end <= 6.0);
			assert!(clump.width_factor.start >= 0.025);
			assert!(clump.width_factor.end <= 0.05);
		}
		Ok(())
	}

	#[test]
	fn patch_wraps_giant_wet_blade_clump() -> Result<()> {
		let MonsterGrassItem::Patch(patch) = MonsterGrassCell::GiantWetBladePatch.item() else {
			anyhow::bail!("expected patch item");
		};
		assert_eq!(patch.clump, GIANT_WET_BLADE);
		assert!(*patch.clump_count.start() >= 3);
		assert!(patch.patch_extent_xz.start >= 1.2);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		// PaleGiantReed (index 3) rejects steepness 0.55; first-fit falls to RedRibbedBlade
		// (index 4), which allows steepness up to 0.60.
		let prepared =
			MonsterGrassCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.55 };
		let outcome = prepared.select_from(
			3,
			Vec3::new(5.0, 0.35, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, MonsterGrassCell::RedRibbedBlade);
			}
			other => anyhow::bail!("expected RedRibbedBlade fallback, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
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
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
