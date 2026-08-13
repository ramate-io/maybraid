//! Common Tufts — well-known sparse-to-moderate grass-clump grove
//! ([RFC-183 §3.4.4.1](../../../../rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/04-well-known-tufts-groves/01-common-tufts/README.md),
//! [#301](https://github.com/ramate-io/maybraid/issues/301)).
//!
//! A lightweight volumetric layer over terrain and ground cover: low 10–50 cm tuft clumps in a
//! few material and shape varietals. All authored data (cell footprint, placement ranges, bucket
//! weights, constraints, palettes, and clump geometry) lives in this module as constants
//! mirroring the RFC blocks.
pub mod variants;

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

/// Authored Common Tufts grove definition.
///
/// Cell footprint is the midpoint of the RFC's `CELL_SIZE_RANGE` (`1.0..3.0`). The offset range
/// is signed and wider than the RFC's nominal `0.0..1.0` (± one cell) so placements break the
/// underlying grid instead of clustering near cell centers; the usual slight deterministic
/// scale variation applies.
pub fn definition() -> GroveDefinition<CommonTuftsCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(2.0),
		placement: GrovePlacementRanges::new(UnitRange::new(0.85, 1.15), UnitRange::new(-2.0, 2.0)),
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
	ShortGreenPatch,
	DryScrubPatch,
	TallWildPatch,
}

/// Typed authored geometry for one common-tufts varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommonTuftsItem {
	Clump(&'static CommonTuftClump),
	Patch(&'static GroveTuftPatch<CommonTuftClump>),
}

/// Authored geometry ranges for one common-tufts grass clump.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonTuftClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**. The RFC's absolute widths describe the
	/// clump footprint, not blade thickness — read literally they render far-too-thick blades.
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

// Modest per-clump shape variation; Braid Grass authors the widest bands of the tuft groves.
const BLADE_COUNT: RangeInclusive<u32> = 6..=10;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=5;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.30);

const SHORT_GREEN: CommonTuftClump = CommonTuftClump {
	height: UnitRange::new(0.10, 0.40),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const DRY_SCRUB: CommonTuftClump = CommonTuftClump {
	height: UnitRange::new(0.15, 0.50),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const TALL_WILD: CommonTuftClump = CommonTuftClump {
	height: UnitRange::new(0.30, 1.0),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

// Patch varietals scatter each clump's blades as loose mounds; they carry most of the
// placed weight, so the single-anchor "cone" clump reads as the rarer silhouette.

const SHORT_GREEN_PATCH: GroveTuftPatch<CommonTuftClump> = GroveTuftPatch {
	clump: SHORT_GREEN,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(0.8, 1.6),
	base_spread: UnitRange::new(0.10, 0.25),
};

const DRY_SCRUB_PATCH: GroveTuftPatch<CommonTuftClump> = GroveTuftPatch {
	clump: DRY_SCRUB,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(0.9, 1.8),
	base_spread: UnitRange::new(0.10, 0.25),
};

const TALL_WILD_PATCH: GroveTuftPatch<CommonTuftClump> = GroveTuftPatch {
	clump: TALL_WILD,
	clump_count: 2..=5,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.15, 0.35),
};

impl CommonTuftsCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.0`; the `None` weight of `13.78` puts the placed share at
	/// `5.0 / 18.78 ≈ 0.266`, inside the RFC's `DENSITY_RANGE` (`0.10..0.35`). Patches carry
	/// `4.0` of the placed weight; single-anchor clumps share the remaining `1.0`.
	pub fn distribution() -> GroveDistribution<Self> {
		let short_green =
			PlacementConstraints::new(UnitRange::new(0.0, 0.80), UnitRange::new(0.0, 0.70));
		let dry_scrub =
			PlacementConstraints::new(UnitRange::new(0.0, 0.90), UnitRange::new(0.0, 0.70));
		let tall_wild =
			PlacementConstraints::new(UnitRange::new(0.0, 0.60), UnitRange::new(0.0, 0.70));
		GroveDistribution::new(vec![
			GroveBucket::none(13.78),
			GroveBucket::placed(0.5, short_green, Self::ShortGreen),
			GroveBucket::placed(0.25, dry_scrub, Self::DryScrub),
			GroveBucket::placed(0.25, tall_wild, Self::TallWild),
			GroveBucket::placed(2.0, short_green, Self::ShortGreenPatch),
			GroveBucket::placed(1.0, dry_scrub, Self::DryScrubPatch),
			GroveBucket::placed(1.0, tall_wild, Self::TallWildPatch),
		])
	}

	/// Authored geometry for this varietal.
	pub fn item(self) -> CommonTuftsItem {
		match self {
			Self::ShortGreen => CommonTuftsItem::Clump(&SHORT_GREEN),
			Self::DryScrub => CommonTuftsItem::Clump(&DRY_SCRUB),
			Self::TallWild => CommonTuftsItem::Clump(&TALL_WILD),
			Self::ShortGreenPatch => CommonTuftsItem::Patch(&SHORT_GREEN_PATCH),
			Self::DryScrubPatch => CommonTuftsItem::Patch(&DRY_SCRUB_PATCH),
			Self::TallWildPatch => CommonTuftsItem::Patch(&TALL_WILD_PATCH),
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
			Self::ShortGreen | Self::ShortGreenPatch => SHORT_GREEN_MIX,
			Self::DryScrub | Self::DryScrubPatch => DRY_SCRUB_MIX,
			Self::TallWild | Self::TallWildPatch => TALL_WILD_MIX,
		}
	}
}


#[cfg(feature = "render")]
mod vc {
	use bevy::prelude::*;
	use chico_vegetation_components::{
		FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::LodSceneLevel;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, CommonTuftsCell, CommonTuftsItem};
	use crate::grove::{
		FlatTerrainSample, GroveCellVariant, GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ,
	};
	use crate::grove::vc_tuft::{
		grow_placed_tuft_params, single_blade_patch_params, stamp_foliage_noise, tuft_grove_stick_nodes,
		TuftGroveBody, TuftGrovePlant, TuftGroveProxyHeights, TUFT_GROVE_STRUCTURAL_HIGH_FACTOR,
		TUFT_GROVE_STRUCTURAL_LOW_FACTOR, TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
	};

	pub const COMMON_TUFTS_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
	pub const COMMON_TUFTS_STRUCTURAL_MEDIUM_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
	pub const COMMON_TUFTS_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct CommonTuftsParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

		#[arg(
			long,
			default_value = "0,1,0.06,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "Foliage Surface Noise",
		)]
		pub foliage_noise: NoiseParams,

		#[arg(skip)]
		pub extent: GroveExtent,

		#[command(flatten, next_help_heading = "Terrain")]
		pub terrain: FlatTerrainSample,

		#[arg(long, default_value_t = 0)]
		pub merge_collections: usize,

		#[arg(long, default_value_t = 100)]
		pub patch_variants: u32,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<CommonTuftsCell>>>,
	}

	impl Default for CommonTuftsParams {
		fn default() -> Self {
			Self {
				grove: GroveFrontend::default(),
				foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain: FlatTerrainSample::default(),
				merge_collections: 0,
				patch_variants: 100,
				resolved_placements: None,
			}
		}
	}

	impl CommonTuftsParams {
		pub fn with_extent(mut self, extent: GroveExtent) -> Self {
			self.extent = extent;
			self
		}

		pub fn with_terrain(mut self, terrain: FlatTerrainSample) -> Self {
			self.terrain = terrain;
			self
		}

		pub fn cell_extent_xz(&self) -> Vec2 {
			self.grove.definition(definition()).cell_extent_xz
		}

		pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
			self.extent.subdivide_xz(self.cell_extent_xz())
		}

		pub fn placements(&self) -> Vec<GroveCellVariant<CommonTuftsCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
		}

		pub fn build(&self) -> CommonTufts {
			let foliage_noise = self.foliage_noise;
			let plants = grow_placed_tuft_params(
				&self.placements(),
				foliage_noise,
				self.merge_collections,
				self.patch_variants,
				|cell, noise| {
					let mix = cell.palette_mix();
					let params = match cell.item() {
						CommonTuftsItem::Clump(clump) => {
							single_blade_patch_params(clump.build_with_noise(noise), foliage_noise)
						}
						CommonTuftsItem::Patch(patch) => {
							stamp_foliage_noise(patch.build_tuft_patch(noise), foliage_noise)
						}
					};
					(params, mix)
				},
			);
			CommonTufts {
				body: TuftGroveBody::from_plants(
					plants,
					&self.extent,
					self.cell_extent_xz(),
					TuftGroveProxyHeights::SHORT,
				),
			}
		}
	}

	#[derive(Clone, Debug, Component)]
	pub struct CommonTufts {
		body: TuftGroveBody,
	}

	impl CommonTufts {
		pub fn plants(&self) -> &[TuftGrovePlant] {
			&self.body.plants
		}
	}

	impl VegetationComponents for CommonTufts {
		fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
			tuft_grove_stick_nodes(level)
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
			self.body.foliage_for_level(level)
		}

		fn structural_lod(&self) -> Option<StructuralLod> {
			Some(self.body.structural_lod())
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	CommonTufts, CommonTuftsParams, COMMON_TUFTS_STRUCTURAL_HIGH_FACTOR,
	COMMON_TUFTS_STRUCTURAL_LOW_FACTOR, COMMON_TUFTS_STRUCTURAL_MEDIUM_FACTOR,
};


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
		let dist = CommonTuftsCell::distribution();
		assert_eq!(dist.len(), 7);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 13.78);
		assert_eq!(dist.buckets[1].item, Some(CommonTuftsCell::ShortGreen));
		assert_eq!(dist.buckets[1].weight, 0.5);
		assert_eq!(dist.buckets[2].item, Some(CommonTuftsCell::DryScrub));
		assert_eq!(dist.buckets[2].weight, 0.25);
		assert_eq!(dist.buckets[3].item, Some(CommonTuftsCell::TallWild));
		assert_eq!(dist.buckets[3].weight, 0.25);
		assert_eq!(dist.buckets[4].item, Some(CommonTuftsCell::ShortGreenPatch));
		assert_eq!(dist.buckets[4].weight, 2.0);
		assert_eq!(dist.buckets[5].item, Some(CommonTuftsCell::DryScrubPatch));
		assert_eq!(dist.buckets[5].weight, 1.0);
		assert_eq!(dist.buckets[6].item, Some(CommonTuftsCell::TallWildPatch));
		assert_eq!(dist.buckets[6].weight, 1.0);
		Ok(())
	}

	#[test]
	fn patches_outweigh_single_clumps() -> Result<()> {
		let placed_weight = |patch: bool| -> f32 {
			CommonTuftsCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| {
						matches!(cell.item(), CommonTuftsItem::Patch(_)) == patch
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
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = CommonTuftsCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.10..=0.35).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn clump_geometry_follows_authored_bands() -> Result<()> {
		for cell in
			[CommonTuftsCell::ShortGreen, CommonTuftsCell::DryScrub, CommonTuftsCell::TallWild]
		{
			let CommonTuftsItem::Clump(clump) = cell.item() else {
				anyhow::bail!("expected clump item for {cell:?}");
			};
			assert!(clump.height.start >= 0.10);
			assert!(clump.height.end <= 1.0);
			assert!(clump.width_factor.start > 0.0);
			assert!(clump.width_factor.end <= 0.05, "blades should stay grass-thin");
		}
		Ok(())
	}

	#[test]
	fn patch_wraps_short_green_clump() -> Result<()> {
		let CommonTuftsItem::Patch(patch) = CommonTuftsCell::ShortGreenPatch.item() else {
			anyhow::bail!("expected patch item");
		};
		assert_eq!(patch.clump, SHORT_GREEN);
		assert!(*patch.clump_count.start() >= 2, "a patch should scatter several clumps");
		assert!(patch.patch_extent_xz.start > 0.0);
		Ok(())
	}

	#[test]
	fn constraint_first_fit_fallback() -> Result<()> {
		// ShortGreen (index 1) rejects elevation 0.85; first-fit falls to DryScrub (index 2).
		let prepared =
			CommonTuftsCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.85, steepness: 0.2 };
		let outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.85, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
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
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
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
