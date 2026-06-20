//! Bush Scrub — well-known sparse tuft-and-bush grove
//! ([RFC-183 §3.4.4.3](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/04-well-known-tufts-groves/03-bush-scrub/README.md),
//! [#303](https://github.com/ramate-io/maybraid/issues/303)).
//!
//! Low irregular scrub mixing 25–50 cm tufts with scaled-down Common High Bush forms. Patch
//! varietals scatter each tuft's blades as loose mounds and carry most of the tuft weight; small
//! bushes stay single-anchor. Forest-layer attachment remains a follow-up.

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
pub use render::{BushScrub, BushScrubStd};

/// RFC `projection_count: Low` — upright rounded low shrubs.
const LOW_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.20, 0.38);
const LOW_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.68, 0.88);

/// RFC `projection_count: VeryLow` — sapling-like upright growth.
const VERY_LOW_PROJECTION_RADIAL: UnitRange = UnitRange::new(0.10, 0.22);
const VERY_LOW_PROJECTION_VERTICAL: UnitRange = UnitRange::new(0.78, 0.92);

/// Authored Bush Scrub grove definition.
///
/// Cell footprint sits in the lower third of the RFC's `CELL_SIZE_RANGE` (`2.0..5.0`) so preview
/// groves read denser than the nominal midpoint grid. The offset range is signed and ± one cell
/// so placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<BushScrubCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.5),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-2.5, 2.5)),
		distribution: BushScrubCell::distribution(),
	}
}

/// Ordered bush-scrub varietals ([RFC-183 §3.4.4.3]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BushScrubCell {
	DryTuft,
	GreenTuft,
	SmallBush,
	SaplingBush,
	DryTuftPatch,
	GreenTuftPatch,
}

/// Typed authored geometry for one bush-scrub varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BushScrubItem {
	Tuft(&'static BushScrubTuft),
	Patch(&'static GroveTuftPatch<BushScrubTuft>),
	Bush(&'static BushScrubBush),
}

/// Authored geometry ranges for one bush-scrub tuft clump.
#[derive(Debug, Clone, PartialEq)]
pub struct BushScrubTuft {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**. The RFC's absolute widths describe the
	/// clump footprint, not blade thickness.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Authored geometry ranges for one scaled-down Common High Bush shrub.
#[derive(Debug, Clone, PartialEq)]
pub struct BushScrubBush {
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

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

const BLADE_COUNT: RangeInclusive<u32> = 6..=10;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=5;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.30);

const DRY_TUFT: BushScrubTuft = BushScrubTuft {
	height: UnitRange::new(0.25, 0.45),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const GREEN_TUFT: BushScrubTuft = BushScrubTuft {
	height: UnitRange::new(0.25, 0.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const SMALL_BUSH: BushScrubBush = BushScrubBush {
	height: UnitRange::new(0.35, 0.80),
	shoot_count: 4..=7,
	branch_depth: 1..=2,
	radial_strength: LOW_PROJECTION_RADIAL,
	vertical_bias: LOW_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.04, 0.08),
};

const SAPLING_BUSH: BushScrubBush = BushScrubBush {
	height: UnitRange::new(0.50, 1.20),
	shoot_count: 3..=5,
	branch_depth: 1..=1,
	radial_strength: VERY_LOW_PROJECTION_RADIAL,
	vertical_bias: VERY_LOW_PROJECTION_VERTICAL,
	leaf_radius: UnitRange::new(0.03, 0.06),
};

// Patch varietals scatter each tuft's blades as loose mounds; they carry most of the tuft
// weight, so the single-anchor "cone" clump reads as the rarer silhouette.

const DRY_TUFT_PATCH: GroveTuftPatch<BushScrubTuft> = GroveTuftPatch {
	clump: DRY_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(0.9, 2.0),
	base_spread: UnitRange::new(0.10, 0.25),
};

const GREEN_TUFT_PATCH: GroveTuftPatch<BushScrubTuft> = GroveTuftPatch {
	clump: GREEN_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(0.9, 2.0),
	base_spread: UnitRange::new(0.12, 0.28),
};

const DRY_TUFT_MIX: PaletteMix = PaletteMix::new(&[PaletteSlot::new("dry_green", "straw_brown")]);
const GREEN_TUFT_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("dark_green", "light_green")]);

const SMALL_BUSH_STICK_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("dry_bark", "gray_brown")]);
const SMALL_BUSH_CANOPY_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("scrub_green", "dry_green")]);
const SAPLING_BUSH_STICK_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("young_bark", "green_brown")]);
const SAPLING_BUSH_CANOPY_MIX: PaletteMix =
	PaletteMix::new(&[PaletteSlot::new("young_green", "light_green")]);

impl BushScrubCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.0` (RFC relative proportions); the `None` weight of `12.0` puts
	/// the placed share at `5.0 / 17.0 ≈ 0.29`, toward the upper end of the RFC's
	/// `DENSITY_RANGE` (`0.10..0.30`) while keeping scrub sparse. Tuft weight (`3.5` total)
	/// leans on patch varietals (`2.8`); single-anchor tufts share the remaining `0.7`. Bush
	/// companions keep their original weights (`1.5`).
	pub fn distribution() -> GroveDistribution<Self> {
		let dry_tuft =
			PlacementConstraints::new(UnitRange::new(0.0, 0.85), UnitRange::new(0.0, 0.75));
		let green_tuft =
			PlacementConstraints::new(UnitRange::new(0.0, 0.85), UnitRange::new(0.0, 0.45));
		let small_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 0.85), UnitRange::new(0.0, 0.65));
		let sapling_bush =
			PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.45));
		GroveDistribution::new(vec![
			GroveBucket::none(12.0),
			GroveBucket::placed(0.4, dry_tuft, Self::DryTuft),
			GroveBucket::placed(0.3, green_tuft, Self::GreenTuft),
			GroveBucket::placed(1.0, small_bush, Self::SmallBush),
			GroveBucket::placed(0.5, sapling_bush, Self::SaplingBush),
			GroveBucket::placed(1.6, dry_tuft, Self::DryTuftPatch),
			GroveBucket::placed(1.2, green_tuft, Self::GreenTuftPatch),
		])
	}

	pub fn item(self) -> BushScrubItem {
		match self {
			Self::DryTuft => BushScrubItem::Tuft(&DRY_TUFT),
			Self::GreenTuft => BushScrubItem::Tuft(&GREEN_TUFT),
			Self::SmallBush => BushScrubItem::Bush(&SMALL_BUSH),
			Self::SaplingBush => BushScrubItem::Bush(&SAPLING_BUSH),
			Self::DryTuftPatch => BushScrubItem::Patch(&DRY_TUFT_PATCH),
			Self::GreenTuftPatch => BushScrubItem::Patch(&GREEN_TUFT_PATCH),
		}
	}

	pub fn palette_mix(self) -> PaletteMix {
		match self {
			Self::DryTuft | Self::DryTuftPatch => DRY_TUFT_MIX,
			Self::GreenTuft | Self::GreenTuftPatch => GREEN_TUFT_MIX,
			Self::SmallBush => SMALL_BUSH_CANOPY_MIX,
			Self::SaplingBush => SAPLING_BUSH_CANOPY_MIX,
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallBush => SMALL_BUSH_STICK_MIX,
			Self::SaplingBush => SAPLING_BUSH_STICK_MIX,
			_ => SMALL_BUSH_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::SmallBush => SMALL_BUSH_CANOPY_MIX,
			Self::SaplingBush => SAPLING_BUSH_CANOPY_MIX,
			_ => GREEN_TUFT_MIX,
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
		let dist = BushScrubCell::distribution();
		assert_eq!(dist.len(), 7);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 12.0);
		assert_eq!(dist.buckets[1].item, Some(BushScrubCell::DryTuft));
		assert_eq!(dist.buckets[1].weight, 0.4);
		assert_eq!(dist.buckets[2].item, Some(BushScrubCell::GreenTuft));
		assert_eq!(dist.buckets[2].weight, 0.3);
		assert_eq!(dist.buckets[3].item, Some(BushScrubCell::SmallBush));
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(BushScrubCell::SaplingBush));
		assert_eq!(dist.buckets[4].weight, 0.5);
		assert_eq!(dist.buckets[5].item, Some(BushScrubCell::DryTuftPatch));
		assert_eq!(dist.buckets[5].weight, 1.6);
		assert_eq!(dist.buckets[6].item, Some(BushScrubCell::GreenTuftPatch));
		assert_eq!(dist.buckets[6].weight, 1.2);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = BushScrubCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.10..=0.30).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn patches_outweigh_single_tufts() -> Result<()> {
		let tuft_weight = |patch: bool| -> f32 {
			BushScrubCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match cell.item() {
						BushScrubItem::Tuft(_) => !patch,
						BushScrubItem::Patch(_) => patch,
						BushScrubItem::Bush(_) => false,
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		assert!(
			tuft_weight(true) > 2.0 * tuft_weight(false),
			"patches should dominate tuft weight"
		);
		Ok(())
	}

	#[test]
	fn tuft_and_bush_placed_weights_match_rfc_ratio() -> Result<()> {
		let weight = |kind: &str| -> f32 {
			BushScrubCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match (kind, cell.item()) {
						("tuft", BushScrubItem::Tuft(_) | BushScrubItem::Patch(_)) => true,
						("bush", BushScrubItem::Bush(_)) => true,
						_ => false,
					})
				})
				.map(|b| b.weight)
				.sum()
		};
		let tuft = weight("tuft");
		let bush = weight("bush");
		assert!((tuft - 3.5).abs() < 1e-4, "expected tuft weight 3.5, got {tuft}");
		assert!((bush - 1.5).abs() < 1e-4, "expected bush weight 1.5, got {bush}");
		Ok(())
	}

	#[test]
	fn tuft_geometry_follows_authored_bands() -> Result<()> {
		for cell in [BushScrubCell::DryTuft, BushScrubCell::GreenTuft] {
			let BushScrubItem::Tuft(tuft) = cell.item() else {
				anyhow::bail!("expected tuft item for {cell:?}");
			};
			assert!(tuft.height.start >= 0.25);
			assert!(tuft.height.end <= 0.50);
			assert!(tuft.width_factor.start > 0.0);
			assert!(tuft.width_factor.end <= 0.05, "blades should stay grass-thin");
		}
		Ok(())
	}

	#[test]
	fn bush_geometry_follows_authored_bands() -> Result<()> {
		let BushScrubItem::Bush(small) = BushScrubCell::SmallBush.item() else {
			anyhow::bail!("expected small bush item");
		};
		assert!(small.height.start >= 0.35);
		assert!(small.height.end <= 0.80);
		assert_eq!(small.shoot_count, 4..=7);
		assert_eq!(small.branch_depth, 1..=2);

		let BushScrubItem::Bush(sapling) = BushScrubCell::SaplingBush.item() else {
			anyhow::bail!("expected sapling bush item");
		};
		assert!(sapling.height.start >= 0.50);
		assert!(sapling.height.end <= 1.20);
		assert_eq!(sapling.shoot_count, 3..=5);
		assert_eq!(sapling.branch_depth, 1..=1);
		Ok(())
	}

	#[test]
	fn patch_wraps_dry_tuft_clump() -> Result<()> {
		let BushScrubItem::Patch(patch) = BushScrubCell::DryTuftPatch.item() else {
			anyhow::bail!("expected patch item");
		};
		assert_eq!(patch.clump, DRY_TUFT);
		assert!(*patch.clump_count.start() >= 2, "a patch should scatter several clumps");
		assert!(patch.patch_extent_xz.start > 0.0);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		// GreenTuft (index 2) rejects steepness 0.50; first-fit falls to SmallBush (index 3),
		// which allows steepness up to 0.65.
		let prepared =
			BushScrubCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.50 };
		let outcome = prepared.select_from(2, Vec3::new(5.0, 0.40, 5.0), 1.0, Cell::from_min_max(Vec3::ZERO, Vec3::ONE), &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, BushScrubCell::SmallBush);
			}
			other => anyhow::bail!("expected SmallBush fallback, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.15 };
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
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
