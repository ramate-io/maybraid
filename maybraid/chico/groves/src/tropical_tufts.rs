//! Tropical Tufts — well-known sparse tuft grove with palm companions
//! ([RFC-183 §3.4.4.5], [#305](https://github.com/ramate-io/maybraid/issues/305)).
//!
//! All authored data (cell footprint, placement ranges, bucket weights, constraints, palettes,
//! and item geometry) lives in this module as constants mirroring the RFC blocks.

use std::ops::RangeInclusive;

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, GroveTuftPatch,
	PaletteMix, PaletteSlot, PlacementConstraints,
};

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
	BrightTuftPatch,
	DeepTuftPatch,
	YellowGreenTuftPatch,
}

/// Typed authored geometry for one tropical-tufts variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TropicalTuftsItem {
	Tuft(&'static TropicalTuftClump),
	PalmBush(&'static TropicalPalmBush),
	Patch(&'static GroveTuftPatch<TropicalTuftClump>),
}

/// Authored geometry ranges for one tropical tuft clump.
#[derive(Debug, Clone, PartialEq)]
pub struct TropicalTuftClump {
	pub height: UnitRange,
	/// Blade width as a **fraction of blade length**; absolute widths render far-too-thick
	/// blades (the RFC widths describe the clump footprint, not blade thickness).
	pub width_factor: UnitRange,
	pub blade_count: RangeInclusive<u32>,
	pub bend_segments: RangeInclusive<u32>,
	pub max_tilt_radians: UnitRange,
}

/// Shared blade thickness band: ~2–4 % of blade length keeps blades grass-thin at any height.
const BLADE_WIDTH_FACTOR: UnitRange = UnitRange::new(0.02, 0.04);

// Modest per-clump shape variation; Braid Grass authors the widest bands of the tuft groves.
const BLADE_COUNT: RangeInclusive<u32> = 6..=12;
const BEND_SEGMENTS: RangeInclusive<u32> = 1..=6;
const MAX_TILT_RADIANS: UnitRange = UnitRange::new(0.15, 0.35);

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
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const DEEP_TUFT: TropicalTuftClump = TropicalTuftClump {
	height: UnitRange::new(0.30, 0.8),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

const YELLOW_GREEN_TUFT: TropicalTuftClump = TropicalTuftClump {
	height: UnitRange::new(0.25, 0.45),
	width_factor: BLADE_WIDTH_FACTOR,
	blade_count: BLADE_COUNT,
	bend_segments: BEND_SEGMENTS,
	max_tilt_radians: MAX_TILT_RADIANS,
};

// Patch varietals scatter each tuft's blades as loose mounds; they carry most of the tuft
// weight, so the single-anchor "cone" clump reads as the rarer silhouette.

const BRIGHT_TUFT_PATCH: GroveTuftPatch<TropicalTuftClump> = GroveTuftPatch {
	clump: BRIGHT_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.15, 0.30),
};

const DEEP_TUFT_PATCH: GroveTuftPatch<TropicalTuftClump> = GroveTuftPatch {
	clump: DEEP_TUFT,
	clump_count: 3..=6,
	patch_extent_xz: UnitRange::new(1.2, 2.4),
	base_spread: UnitRange::new(0.15, 0.35),
};

const YELLOW_GREEN_TUFT_PATCH: GroveTuftPatch<TropicalTuftClump> = GroveTuftPatch {
	clump: YELLOW_GREEN_TUFT,
	clump_count: 2..=5,
	patch_extent_xz: UnitRange::new(1.0, 2.0),
	base_spread: UnitRange::new(0.15, 0.30),
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
	///
	/// Tuft weight (`5.5` total) leans on the patch varietals (`4.4`); single-anchor clumps
	/// share the remaining `1.1`. Palm companions keep their original weights.
	pub fn distribution() -> GroveDistribution<Self> {
		let bright =
			PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.35));
		let lowland =
			PlacementConstraints::new(UnitRange::new(0.0, 0.65), UnitRange::new(0.0, 0.75));
		let juvenile =
			PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.75));
		GroveDistribution::new(vec![
			GroveBucket::none(10.0),
			GroveBucket::placed(0.5, bright, Self::BrightTuft),
			GroveBucket::placed(0.35, lowland, Self::DeepTuft),
			GroveBucket::placed(0.25, lowland, Self::YellowGreenTuft),
			GroveBucket::placed(0.75, lowland, Self::SmallPalmBush),
			GroveBucket::placed(0.35, juvenile, Self::JuvenilePalmBush),
			GroveBucket::placed(2.0, bright, Self::BrightTuftPatch),
			GroveBucket::placed(1.5, lowland, Self::DeepTuftPatch),
			GroveBucket::placed(0.9, lowland, Self::YellowGreenTuftPatch),
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
			Self::BrightTuftPatch => TropicalTuftsItem::Patch(&BRIGHT_TUFT_PATCH),
			Self::DeepTuftPatch => TropicalTuftsItem::Patch(&DEEP_TUFT_PATCH),
			Self::YellowGreenTuftPatch => TropicalTuftsItem::Patch(&YELLOW_GREEN_TUFT_PATCH),
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
			Self::BrightTuft | Self::BrightTuftPatch => BRIGHT_TUFT_MIX,
			Self::DeepTuft | Self::DeepTuftPatch => DEEP_TUFT_MIX,
			Self::YellowGreenTuft | Self::YellowGreenTuftPatch => YELLOW_GREEN_TUFT_MIX,
			Self::SmallPalmBush => SMALL_PALM_BUSH_MIX,
			Self::JuvenilePalmBush => JUVENILE_PALM_BUSH_MIX,
		}
	}
}


#[cfg(feature = "render")]
mod vc {
	use bevy::prelude::*;
	use chico_sbs_trees::{PalmBush, PalmBushParams};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::LodSceneLevel;
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, TropicalTuftsCell, TropicalTuftsItem};
	use crate::grove::{
		flatten_foliage_nodes, frond_material_from_palette, placement_noise, FlatTerrainSample,
		GroveCellVariant, GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ,
	};
	use crate::grove::vc_tuft::{
		grow_tuft_plants, material_from_palette, patch_variant_index, single_blade_patch_params,
		stamp_foliage_noise, tuft_grove_stick_nodes, unit_plant_from_params, variant_noise,
		TuftGroveBody, TuftGrovePlant, TuftGroveProxyHeights, TUFT_GROVE_STRUCTURAL_HIGH_FACTOR,
		TUFT_GROVE_STRUCTURAL_LOW_FACTOR, TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
	};

	pub const TROPICAL_TUFTS_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
	pub const TROPICAL_TUFTS_STRUCTURAL_MEDIUM_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
	pub const TROPICAL_TUFTS_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct TropicalTuftsParams {
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
		resolved_placements: Option<Vec<GroveCellVariant<TropicalTuftsCell>>>,
	}

	impl Default for TropicalTuftsParams {
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

	impl TropicalTuftsParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<TropicalTuftsCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
		}

		pub fn build(&self) -> TropicalTufts {
			let foliage_noise = self.foliage_noise;
			let variants = self.patch_variants.max(1);
			let mut tuft_grown = Vec::new();
			let mut palms = Vec::new();
			for placed in self.placements() {
				let mix = placed.variant.palette_mix();
				match placed.variant.item() {
					TropicalTuftsItem::Tuft(clump) => {
						let variant = patch_variant_index(placed.position, variants);
						let noise = variant_noise(foliage_noise, variant);
						let params = single_blade_patch_params(
							clump.build_with_noise(noise),
							foliage_noise,
						);
						let material =
							material_from_palette(mix, placed.position, foliage_noise);
						tuft_grown.push(unit_plant_from_params(
							params,
							variant,
							placed.position,
							placed.scale,
							material,
						));
					}
					TropicalTuftsItem::Patch(patch) => {
						let variant = patch_variant_index(placed.position, variants);
						let noise = variant_noise(foliage_noise, variant);
						let params =
							stamp_foliage_noise(patch.build_tuft_patch(noise), foliage_noise);
						let material =
							material_from_palette(mix, placed.position, foliage_noise);
						tuft_grown.push(unit_plant_from_params(
							params,
							variant,
							placed.position,
							placed.scale,
							material,
						));
					}
					TropicalTuftsItem::PalmBush(palm) => {
						let noise = placement_noise(foliage_noise, placed.position);
						let geometry = palm.build_with_noise(noise);
						let mut params = PalmBushParams::default();
						params.geometry = geometry;
						let material =
							material_from_palette(mix, placed.position, foliage_noise);
						let frond_material =
							frond_material_from_palette(Some(mix), noise.seed);
						palms.push(TropicalTuftsPalm {
							placement: Placement::new(placed.position, 0.0)
								.with_scale(Vec3::splat(placed.scale.max(1e-4))),
							bush: params.build(),
							material,
							frond_material,
						});
					}
				}
			}
			TropicalTufts {
				body: TuftGroveBody::from_plants(
					grow_tuft_plants(tuft_grown, self.merge_collections),
					&self.extent,
					self.cell_extent_xz(),
					TuftGroveProxyHeights::SHORT,
				),
				palms,
			}
		}
	}

	#[derive(Clone)]
	struct TropicalTuftsPalm {
		placement: Placement,
		bush: PalmBush,
		material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct TropicalTufts {
		body: TuftGroveBody,
		palms: Vec<TropicalTuftsPalm>,
	}

	impl TropicalTufts {
		pub fn plants(&self) -> &[TuftGrovePlant] {
			&self.body.plants
		}
	}

	impl VegetationComponents for TropicalTufts {
		fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
			tuft_grove_stick_nodes(level)
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
			let mut nodes = self.body.foliage_for_level(level).flatten();
			// Palm companions: High/Medium fronds; Low/UltraLow layered balls from PalmBush.
			let palm_level = match level {
				LodSceneLevel::Medium => LodSceneLevel::High,
				other => other,
			};
			for palm in &self.palms {
				nodes.extend(flatten_foliage_nodes(
					&palm.bush,
					palm.placement,
					&palm.material,
					&palm.frond_material,
					palm_level,
				));
			}
			Layers::from_free(nodes)
		}

		fn structural_lod(&self) -> Option<StructuralLod> {
			Some(self.body.structural_lod())
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	TropicalTufts, TropicalTuftsParams, TROPICAL_TUFTS_STRUCTURAL_HIGH_FACTOR,
	TROPICAL_TUFTS_STRUCTURAL_LOW_FACTOR, TROPICAL_TUFTS_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = TropicalTuftsCell::distribution();
		assert_eq!(dist.len(), 9);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 10.0);
		assert_eq!(dist.buckets[1].item, Some(TropicalTuftsCell::BrightTuft));
		assert_eq!(dist.buckets[1].weight, 0.5);
		assert_eq!(dist.buckets[2].weight, 0.35);
		assert_eq!(dist.buckets[3].weight, 0.25);
		assert_eq!(dist.buckets[4].item, Some(TropicalTuftsCell::SmallPalmBush));
		assert_eq!(dist.buckets[4].weight, 0.75);
		assert_eq!(dist.buckets[5].item, Some(TropicalTuftsCell::JuvenilePalmBush));
		assert_eq!(dist.buckets[5].weight, 0.35);
		assert_eq!(dist.buckets[6].item, Some(TropicalTuftsCell::BrightTuftPatch));
		assert_eq!(dist.buckets[6].weight, 2.0);
		assert_eq!(dist.buckets[7].item, Some(TropicalTuftsCell::DeepTuftPatch));
		assert_eq!(dist.buckets[7].weight, 1.5);
		assert_eq!(dist.buckets[8].item, Some(TropicalTuftsCell::YellowGreenTuftPatch));
		assert_eq!(dist.buckets[8].weight, 0.9);
		Ok(())
	}

	#[test]
	fn patches_outweigh_single_tufts() -> Result<()> {
		let tuft_weight = |patch: bool| -> f32 {
			TropicalTuftsCell::distribution()
				.buckets
				.iter()
				.filter(|b| {
					b.item.is_some_and(|cell| match cell.item() {
						TropicalTuftsItem::Tuft(_) => !patch,
						TropicalTuftsItem::Patch(_) => patch,
						TropicalTuftsItem::PalmBush(_) => false,
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
	fn variants_map_to_typed_items() -> Result<()> {
		assert!(matches!(TropicalTuftsCell::BrightTuft.item(), TropicalTuftsItem::Tuft(_)));
		let TropicalTuftsItem::PalmBush(palm) = TropicalTuftsCell::SmallPalmBush.item() else {
			anyhow::bail!("expected palm bush item");
		};
		assert_eq!(palm.frond_count, 4..=7);
		let TropicalTuftsItem::Patch(patch) = TropicalTuftsCell::BrightTuftPatch.item() else {
			anyhow::bail!("expected patch item");
		};
		assert_eq!(patch.clump, BRIGHT_TUFT);
		Ok(())
	}

	#[test]
	fn first_fit_from_placed_bucket_places_variant() -> Result<()> {
		let prepared =
			TropicalTuftsCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.4, steepness: 0.1 };
		let outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.4, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
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
