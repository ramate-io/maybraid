//! Wild Grass — well-known dense colorful tall-tuft grove
//! ([RFC-183 §3.4.4.4](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/04-well-known-tufts-groves/04-wild-grass/README.md),
//! [#304](https://github.com/ramate-io/maybraid/issues/304)).
//!
//! Dense tall grass (50–100 cm) with strong palette variation across six color families.
//! All authored data lives in this module as constants mirroring the RFC blocks.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{WildGrass, WildGrassStd};

/// Authored Wild Grass grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`1.0..2.5`). The offset range
/// is signed and wider than the RFC's nominal `0.0..1.0` (± one cell) so placements break the
/// underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<WildGrassCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(1.75),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-1.75, 1.75),
		),
		distribution: WildGrassCell::distribution(),
	}
}

/// Ordered wild-grass varietals ([RFC-183 §3.4.4.4]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WildGrassCell {
	MeadowGreen,
	GoldenGrass,
	RedPrairie,
	BlueTropical,
	PaleField,
	BloomingGrass,
	MeadowGreenPatch,
	GoldenGrassPatch,
	RedPrairiePatch,
	BlueTropicalPatch,
	PaleFieldPatch,
	BloomingGrassPatch,
}

/// Typed authored geometry for one wild-grass varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WildGrassItem {
	Clump(&'static WildGrassClump),
	Patch(&'static GroveTuftPatch<WildGrassClump>),
}

/// Authored geometry ranges for one wild-grass blade clump.
#[derive(Debug, Clone, PartialEq)]
pub struct WildGrassClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**. The RFC's absolute widths describe the
	/// clump footprint, not blade thickness.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

// Denser tall grass than the short-tuft groves; shape bands sit between Common and Braid.
const BLADE_COUNT: RangeInclusive<u32> = 8..=14;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=6;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.35);

const MEADOW_GREEN: WildGrassClump = WildGrassClump {
	height: UnitRange::new(0.50, 0.90),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const GOLDEN_GRASS: WildGrassClump = WildGrassClump {
	height: UnitRange::new(0.60, 1.00),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const RED_PRAIRIE: WildGrassClump = WildGrassClump {
	height: UnitRange::new(0.60, 1.00),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const BLUE_TROPICAL: WildGrassClump = WildGrassClump {
	height: UnitRange::new(0.60, 0.95),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const PALE_FIELD: WildGrassClump = WildGrassClump {
	height: UnitRange::new(0.50, 0.85),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const BLOOMING_GRASS: WildGrassClump = WildGrassClump {
	height: UnitRange::new(0.50, 0.90),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

// Patch varietals scatter each clump's blades as loose mounds; they carry most of the placed
// weight so the single-anchor "cone" clump reads as the rarer silhouette.

const MEADOW_GREEN_PATCH: GroveTuftPatch<WildGrassClump> = GroveTuftPatch {
	clump: MEADOW_GREEN,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.15, 0.30),
};

const GOLDEN_GRASS_PATCH: GroveTuftPatch<WildGrassClump> = GroveTuftPatch {
	clump: GOLDEN_GRASS,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.2, 2.4),
	base_spread: UnitRange::new(0.20, 0.40),
};

const RED_PRAIRIE_PATCH: GroveTuftPatch<WildGrassClump> = GroveTuftPatch {
	clump: RED_PRAIRIE,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.2, 2.4),
	base_spread: UnitRange::new(0.20, 0.40),
};

const BLUE_TROPICAL_PATCH: GroveTuftPatch<WildGrassClump> = GroveTuftPatch {
	clump: BLUE_TROPICAL,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.2),
	base_spread: UnitRange::new(0.15, 0.35),
};

const PALE_FIELD_PATCH: GroveTuftPatch<WildGrassClump> = GroveTuftPatch {
	clump: PALE_FIELD,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.15, 0.30),
};

const BLOOMING_GRASS_PATCH: GroveTuftPatch<WildGrassClump> = GroveTuftPatch {
	clump: BLOOMING_GRASS,
	clump_count: 4..=7,
	patch_extent_xz: UnitRange::new(1.0, 2.2),
	base_spread: UnitRange::new(0.15, 0.35),
};

impl WildGrassCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `7.0` (RFC relative proportions); the `None` weight of `1.0` puts
	/// the placed share at `7.0 / 8.0 = 0.875`, inside the RFC's `DENSITY_RANGE`
	/// (`0.65..0.90`). Patches carry `5.6` of the placed weight; single-anchor clumps share
	/// the remaining `1.4`.
	pub fn distribution() -> GroveDistribution<Self> {
		let meadow =
			PlacementConstraints::new(UnitRange::new(0.0, 0.60), UnitRange::new(0.0, 0.65));
		let golden =
			PlacementConstraints::new(UnitRange::new(0.0, 0.70), UnitRange::new(0.0, 0.55));
		let prairie =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.35));
		let tropical =
			PlacementConstraints::new(UnitRange::new(0.0, 0.40), UnitRange::new(0.0, 0.35));
		let pale_field =
			PlacementConstraints::new(UnitRange::new(0.0, 0.60), UnitRange::new(0.0, 0.35));
		let blooming =
			PlacementConstraints::new(UnitRange::new(0.0, 0.70), UnitRange::new(0.0, 0.35));
		GroveDistribution::new(vec![
			GroveBucket::none(1.0),
			GroveBucket::placed(0.4, meadow, Self::MeadowGreen),
			GroveBucket::placed(0.3, golden, Self::GoldenGrass),
			GroveBucket::placed(0.2, prairie, Self::RedPrairie),
			GroveBucket::placed(0.16, tropical, Self::BlueTropical),
			GroveBucket::placed(0.2, pale_field, Self::PaleField),
			GroveBucket::placed(0.14, blooming, Self::BloomingGrass),
			GroveBucket::placed(1.6, meadow, Self::MeadowGreenPatch),
			GroveBucket::placed(1.2, golden, Self::GoldenGrassPatch),
			GroveBucket::placed(0.8, prairie, Self::RedPrairiePatch),
			GroveBucket::placed(0.64, tropical, Self::BlueTropicalPatch),
			GroveBucket::placed(0.8, pale_field, Self::PaleFieldPatch),
			GroveBucket::placed(0.56, blooming, Self::BloomingGrassPatch),
		])
	}

	/// Authored geometry for this varietal.
	pub fn item(self) -> WildGrassItem {
		match self {
			Self::MeadowGreen => WildGrassItem::Clump(&MEADOW_GREEN),
			Self::GoldenGrass => WildGrassItem::Clump(&GOLDEN_GRASS),
			Self::RedPrairie => WildGrassItem::Clump(&RED_PRAIRIE),
			Self::BlueTropical => WildGrassItem::Clump(&BLUE_TROPICAL),
			Self::PaleField => WildGrassItem::Clump(&PALE_FIELD),
			Self::BloomingGrass => WildGrassItem::Clump(&BLOOMING_GRASS),
			Self::MeadowGreenPatch => WildGrassItem::Patch(&MEADOW_GREEN_PATCH),
			Self::GoldenGrassPatch => WildGrassItem::Patch(&GOLDEN_GRASS_PATCH),
			Self::RedPrairiePatch => WildGrassItem::Patch(&RED_PRAIRIE_PATCH),
			Self::BlueTropicalPatch => WildGrassItem::Patch(&BLUE_TROPICAL_PATCH),
			Self::PaleFieldPatch => WildGrassItem::Patch(&PALE_FIELD_PATCH),
			Self::BloomingGrassPatch => WildGrassItem::Patch(&BLOOMING_GRASS_PATCH),
		}
	}

	/// Authored palette ranges for this varietal.
	pub fn palette_mix(self) -> PaletteMix {
		const MEADOW_GREEN_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("deep_green", "light_green"),
			PaletteSlot::new("yellow_green", "spring_green"),
			PaletteSlot::new("olive_green", "dark_green"),
		]);
		const GOLDEN_GRASS_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("yellow_green", "gold"),
			PaletteSlot::new("pale_straw", "warm_yellow"),
			PaletteSlot::new("dry_green", "light_brown"),
		]);
		const RED_PRAIRIE_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("red_brown", "deep_rust"),
			PaletteSlot::new("orange_brown", "dark_red"),
			PaletteSlot::new("dry_green", "yellow_green"),
		]);
		const BLUE_TROPICAL_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("blue_green", "aqua_green"),
			PaletteSlot::new("pale_teal", "sky_blue"),
			PaletteSlot::new("bright_green", "light_green"),
		]);
		const PALE_FIELD_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("pale_straw", "dry_green"),
			PaletteSlot::new("cream_yellow", "light_brown"),
			PaletteSlot::new("silver_green", "olive_green"),
		]);
		const BLOOMING_GRASS_MIX: PaletteMix = PaletteMix::new(&[
			PaletteSlot::new("green", "flower_flecked"),
			PaletteSlot::new("yellow_green", "soft_pink"),
			PaletteSlot::new("light_green", "white_bloom"),
			PaletteSlot::new("deep_green", "violet_flecked"),
		]);
		match self {
			Self::MeadowGreen | Self::MeadowGreenPatch => MEADOW_GREEN_MIX,
			Self::GoldenGrass | Self::GoldenGrassPatch => GOLDEN_GRASS_MIX,
			Self::RedPrairie | Self::RedPrairiePatch => RED_PRAIRIE_MIX,
			Self::BlueTropical | Self::BlueTropicalPatch => BLUE_TROPICAL_MIX,
			Self::PaleField | Self::PaleFieldPatch => PALE_FIELD_MIX,
			Self::BloomingGrass | Self::BloomingGrassPatch => BLOOMING_GRASS_MIX,
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
		let dist = WildGrassCell::distribution();
		assert_eq!(dist.len(), 13);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 1.0);
		assert_eq!(dist.buckets[1].item, Some(WildGrassCell::MeadowGreen));
		assert_eq!(dist.buckets[1].weight, 0.4);
		assert_eq!(dist.buckets[2].item, Some(WildGrassCell::GoldenGrass));
		assert_eq!(dist.buckets[2].weight, 0.3);
		assert_eq!(dist.buckets[3].item, Some(WildGrassCell::RedPrairie));
		assert_eq!(dist.buckets[3].weight, 0.2);
		assert_eq!(dist.buckets[4].item, Some(WildGrassCell::BlueTropical));
		assert_eq!(dist.buckets[4].weight, 0.16);
		assert_eq!(dist.buckets[5].item, Some(WildGrassCell::PaleField));
		assert_eq!(dist.buckets[5].weight, 0.2);
		assert_eq!(dist.buckets[6].item, Some(WildGrassCell::BloomingGrass));
		assert_eq!(dist.buckets[6].weight, 0.14);
		assert_eq!(dist.buckets[7].item, Some(WildGrassCell::MeadowGreenPatch));
		assert_eq!(dist.buckets[7].weight, 1.6);
		assert_eq!(dist.buckets[8].item, Some(WildGrassCell::GoldenGrassPatch));
		assert_eq!(dist.buckets[8].weight, 1.2);
		assert_eq!(dist.buckets[9].item, Some(WildGrassCell::RedPrairiePatch));
		assert_eq!(dist.buckets[9].weight, 0.8);
		assert_eq!(dist.buckets[10].item, Some(WildGrassCell::BlueTropicalPatch));
		assert_eq!(dist.buckets[10].weight, 0.64);
		assert_eq!(dist.buckets[11].item, Some(WildGrassCell::PaleFieldPatch));
		assert_eq!(dist.buckets[11].weight, 0.8);
		assert_eq!(dist.buckets[12].item, Some(WildGrassCell::BloomingGrassPatch));
		assert_eq!(dist.buckets[12].weight, 0.56);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = WildGrassCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.65..=0.90).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn patches_outweigh_single_clumps() -> Result<()> {
		let placed_weight = |patch: bool| -> f32 {
			WildGrassCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item
						.is_some_and(|cell| matches!(cell.item(), WildGrassItem::Patch(_)) == patch)
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
			WildGrassCell::MeadowGreen,
			WildGrassCell::GoldenGrass,
			WildGrassCell::RedPrairie,
			WildGrassCell::BlueTropical,
			WildGrassCell::PaleField,
			WildGrassCell::BloomingGrass,
		] {
			let WildGrassItem::Clump(clump) = cell.item() else {
				anyhow::bail!("expected clump item for {cell:?}");
			};
			assert!(clump.height.start >= 0.50);
			assert!(clump.height.end <= 1.0);
			assert!(clump.width_factor.start > 0.0);
			assert!(clump.width_factor.end <= 0.05, "blades should stay grass-thin");
		}
		Ok(())
	}

	#[test]
	fn patch_wraps_meadow_green_clump() -> Result<()> {
		let WildGrassItem::Patch(patch) = WildGrassCell::MeadowGreenPatch.item() else {
			anyhow::bail!("expected patch item");
		};
		assert_eq!(patch.clump, MEADOW_GREEN);
		assert!(*patch.clump_count.start() >= 2, "a patch should scatter several clumps");
		assert!(patch.patch_extent_xz.start > 0.0);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		// BlueTropical (index 4) rejects elevation 0.45; first-fit falls to PaleField (index 5).
		let prepared =
			WildGrassCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.45, steepness: 0.20 };
		let outcome = prepared.select_from(4, Vec3::new(5.0, 0.45, 5.0), 1.0, Cell::from_min_max(Vec3::ZERO, Vec3::ONE), &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, WildGrassCell::PaleField);
			}
			other => anyhow::bail!("expected PaleField fallback, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(20.0, 1.0, 20.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
