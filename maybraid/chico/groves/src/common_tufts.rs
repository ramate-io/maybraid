//! Common Tufts — well-known sparse-to-moderate grass-clump grove
//! ([RFC-183 §3.4.4.1](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/04-well-known-tufts-groves/01-common-tufts/README.md),
//! [#301](https://github.com/ramate-io/maybraid/issues/301)).
//!
//! A lightweight volumetric layer over terrain and ground cover: low 10–50 cm tuft clumps in a
//! few material and shape varietals. All authored data (cell footprint, placement ranges, bucket
//! weights, constraints, palettes, and clump geometry) lives in this module as constants
//! mirroring the RFC blocks.

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix,
	PaletteSlot, PlacementConstraints,
};

#[cfg(feature = "render")]
mod render;
#[cfg(feature = "render")]
pub use render::{CommonTufts, CommonTuftsStd};

/// Authored Common Tufts grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`1.0..3.0`). The offset range
/// is signed and wider than the RFC's nominal `0.0..1.0` (± one cell) so placements break the
/// underlying grid instead of clustering near cell centers; the usual slight deterministic
/// scale variation applies.
pub fn definition() -> GroveDefinition<CommonTuftsCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-2.0, 2.0),
		),
		distribution: CommonTuftsCell::distribution(),
	}
}

/// Ordered common-tufts varietals ([RFC-183 §3.4.4.1]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonTuftsCell {
	ShortGreen,
	DryScrub,
	TallWild,
}

/// Authored geometry ranges for one common-tufts grass clump.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonTuftClump {
	pub height: UnitRange,
	pub width: UnitRange,
}

const SHORT_GREEN: CommonTuftClump = CommonTuftClump {
	height: UnitRange::new(0.10, 0.25),
	width: UnitRange::new(0.08, 0.20),
};

const DRY_SCRUB: CommonTuftClump = CommonTuftClump {
	height: UnitRange::new(0.15, 0.40),
	width: UnitRange::new(0.08, 0.25),
};

const TALL_WILD: CommonTuftClump = CommonTuftClump {
	height: UnitRange::new(0.30, 0.50),
	width: UnitRange::new(0.12, 0.30),
};

impl CommonTuftsCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `4.0`; the `None` weight of `13.78` puts the placed share at
	/// `4.0 / 17.78 ≈ 0.225`, the midpoint of the RFC's `DENSITY_RANGE` (`0.10..0.35`).
	pub fn distribution() -> GroveDistribution<Self> {
		let short_green =
			PlacementConstraints::new(UnitRange::new(0.0, 0.80), UnitRange::new(0.0, 0.70));
		let dry_scrub =
			PlacementConstraints::new(UnitRange::new(0.0, 0.90), UnitRange::new(0.0, 0.70));
		let tall_wild =
			PlacementConstraints::new(UnitRange::new(0.0, 0.60), UnitRange::new(0.0, 0.70));
		GroveDistribution::new(vec![
			GroveBucket::none(13.78),
			GroveBucket::placed(2.0, short_green, Self::ShortGreen),
			GroveBucket::placed(1.0, dry_scrub, Self::DryScrub),
			GroveBucket::placed(1.0, tall_wild, Self::TallWild),
		])
	}

	/// Authored geometry ranges for this varietal.
	pub fn clump(self) -> &'static CommonTuftClump {
		match self {
			Self::ShortGreen => &SHORT_GREEN,
			Self::DryScrub => &DRY_SCRUB,
			Self::TallWild => &TALL_WILD,
		}
	}

	/// Authored palette ranges for this varietal (one RFC slot each).
	pub fn palette_mix(self) -> PaletteMix {
		const SHORT_GREEN_MIX: PaletteMix =
			PaletteMix::new(&[PaletteSlot::new("dark_green", "light_green")]);
		const DRY_SCRUB_MIX: PaletteMix =
			PaletteMix::new(&[PaletteSlot::new("vibrant_yellow_green", "dry_yellow_green")]);
		const TALL_WILD_MIX: PaletteMix =
			PaletteMix::new(&[PaletteSlot::new("green", "pale_green")]);
		match self {
			Self::ShortGreen => SHORT_GREEN_MIX,
			Self::DryScrub => DRY_SCRUB_MIX,
			Self::TallWild => TALL_WILD_MIX,
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
		let dist = CommonTuftsCell::distribution();
		assert_eq!(dist.len(), 4);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 13.78);
		assert_eq!(dist.buckets[1].item, Some(CommonTuftsCell::ShortGreen));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(CommonTuftsCell::DryScrub));
		assert_eq!(dist.buckets[2].weight, 1.0);
		assert_eq!(dist.buckets[3].item, Some(CommonTuftsCell::TallWild));
		assert_eq!(dist.buckets[3].weight, 1.0);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = CommonTuftsCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 =
			dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.10..=0.35).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn clump_heights_follow_rfc_band() -> Result<()> {
		for cell in
			[CommonTuftsCell::ShortGreen, CommonTuftsCell::DryScrub, CommonTuftsCell::TallWild]
		{
			let clump = cell.clump();
			assert!(clump.height.start >= 0.10);
			assert!(clump.height.end <= 0.50);
		}
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		// ShortGreen (index 1) rejects elevation 0.85; first-fit falls to DryScrub (index 2).
		let prepared = CommonTuftsCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.85, steepness: 0.2 };
		let outcome = prepared.select_from(1, Vec3::new(5.0, 0.85, 5.0), 1.0, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, CommonTuftsCell::DryScrub);
			}
			other => anyhow::bail!("expected DryScrub fallback, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		// Match the frontend default: cellular per-cell hash values for placement draws.
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove =
			Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let placements = grove.populate(&extent, &terrain);
		assert!(!placements.is_empty());

		// With ±cell offsets, a healthy share of placements should sit far from any cell
		// center; near-center clustering is what reads as a grid.
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
