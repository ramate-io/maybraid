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

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{HighBushShoots, HighBushShootsParams, TuftPatch};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, BushScrubCell, BushScrubItem};
	use crate::grove::vc_tuft::{
		material_from_palette, patch_variant_index, single_blade_patch_params, stamp_foliage_noise,
		unit_plant_from_params, variant_noise, TUFT_GROVE_STRUCTURAL_HIGH_FACTOR,
		TUFT_GROVE_STRUCTURAL_LOW_FACTOR, TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
	};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_flattened_plant_chunk, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const BUSH_SCRUB_STRUCTURAL_HIGH_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_HIGH_FACTOR;
	pub const BUSH_SCRUB_STRUCTURAL_MEDIUM_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR;
	pub const BUSH_SCRUB_STRUCTURAL_LOW_FACTOR: f32 = TUFT_GROVE_STRUCTURAL_LOW_FACTOR;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct BushScrubParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

		#[arg(
			long,
			default_value = "0,1.0,1.0,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "The noise applied to the chains of sticks in the bushes",
		)]
		pub bush_chain_noise: NoiseParams,

		#[arg(
			long,
			default_value = "0,1.0,0.05,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "Stick Surface Noise",
		)]
		pub stick_surface_noise: NoiseParams,

		#[arg(
			long,
			default_value = "0,1.0,0.06,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "Leaf Surface Noise",
		)]
		pub leaf_surface_noise: NoiseParams,

		#[arg(skip)]
		pub extent: GroveExtent,

		#[command(flatten, next_help_heading = "Terrain")]
		pub terrain: FlatTerrainSample,

		#[arg(long, default_value_t = 100)]
		pub patch_variants: u32,

		/// Number of unit-height bush archetypes (`unit_from_num(0..n)`). Caps unique
		/// merged-mesh handles for High/Medium bushes.
		#[arg(long, default_value_t = 100)]
		pub tree_variants: u32,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<BushScrubCell>>>,
	}

	impl Default for BushScrubParams {
		fn default() -> Self {
			Self {
				grove: GroveFrontend::default(),
				bush_chain_noise: NoiseParams::from_scalar(0.0, 1.0, 1.0, 1),
				stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
				leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain: FlatTerrainSample::default(),
				patch_variants: 100,
				tree_variants: 100,
				resolved_placements: None,
			}
		}
	}

	impl BushScrubParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<BushScrubCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<BushScrubCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> BushScrub {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> BushScrub {
			BushScrub::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.bush_chain_noise,
				self.leaf_surface_noise,
				self.patch_variants,
				self.tree_variants,
				&self.extent,
			)
		}
	}

	#[derive(Clone)]
	enum BushScrubKind {
		Tuft(Arc<TuftPatch>),
		Bush(Arc<HighBushShoots>),
	}

	#[derive(Clone)]
	struct BushScrubPlant {
		placement: Placement,
		kind: BushScrubKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct BushScrub {
		plants: Arc<[BushScrubPlant]>,
		structural_center: Vec3,
		footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl BushScrub {
		pub fn from_placements(
			placements: &[GroveCellVariant<BushScrubCell>],
			grove_noise: NoiseParams,
			bush_chain_noise: NoiseParams,
			leaf_surface_noise: NoiseParams,
			patch_variants: u32,
			tree_variants: u32,
			extent: &GroveExtent,
		) -> Self {
			let patch_variants = patch_variants.max(1);
			let tree_variants = tree_variants.max(1);
			let plants: Arc<[BushScrubPlant]> = placements
				.iter()
				.map(|placed| {
					grow_plant(
						placed,
						grove_noise,
						bush_chain_noise,
						leaf_surface_noise,
						patch_variants,
						tree_variants,
					)
				})
				.collect::<Vec<_>>()
				.into();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		pub fn is_empty(&self) -> bool {
			self.plants.is_empty()
		}

		fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			if self.plants.is_empty() {
				return Vec::new();
			}
			let n = self.plants.len();
			let plants = Arc::clone(&self.plants);
			let prev = *lod_ref.previous_transform;
			let curr = *lod_ref.current_transform;
			let bounds = *lod_ref.bounds;
			let entity = lod_ref.entity;
			let mut index = 0usize;
			vec![SceneChunk::lazy(n as u32, n, move || {
				if index >= plants.len() {
					return None;
				}
				let plant = &plants[index];
				index += 1;
				let plant_lod = LodRef {
					entity,
					previous_transform: &prev,
					current_transform: &curr,
					bounds: &bounds,
				};
				Some(match &plant.kind {
					BushScrubKind::Tuft(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					BushScrubKind::Bush(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
				})
			})]
		}

		fn canopy_sites(&self) -> Vec<CanopyProxySite> {
			self.plants
				.iter()
				.filter_map(|plant| match &plant.kind {
					BushScrubKind::Bush(t) => {
						canopy_proxy_site(t, plant.placement, &plant.ball_material)
					}
					BushScrubKind::Tuft(t) => {
						Some(tuft_proxy_site(t, plant.placement, &plant.ball_material))
					}
				})
				.collect()
		}
	}

	fn tuft_proxy_site(
		patch: &TuftPatch,
		placement: Placement,
		material: &MaterialRef,
	) -> CanopyProxySite {
		let scale = placement.scale.abs().max_element().max(1e-4);
		let height = (patch.shape.blade_length * scale).max(0.15);
		let footprint = (patch.patch_extent_xz * 0.5 * scale).max(height * 0.35);
		CanopyProxySite::from_radius(
			placement.translation + Vec3::Y * (height * 0.4),
			footprint.max(0.25),
			material.clone(),
		)
	}

	fn grow_plant(
		placed: &GroveCellVariant<BushScrubCell>,
		grove_noise: NoiseParams,
		bush_chain_noise: NoiseParams,
		leaf_surface_noise: NoiseParams,
		patch_variants: u32,
		tree_variants: u32,
	) -> BushScrubPlant {
		match placed.variant.item() {
			BushScrubItem::Tuft(tuft) => {
				let variant = patch_variant_index(placed.position, patch_variants);
				let noise = variant_noise(leaf_surface_noise, variant);
				let params =
					single_blade_patch_params(tuft.build_with_noise(noise), leaf_surface_noise);
				let material = material_from_palette(
					placed.variant.palette_mix(),
					placed.position,
					leaf_surface_noise,
				);
				let (placement, patch, material) = unit_plant_from_params(
					params,
					variant,
					placed.position,
					placed.scale,
					material,
				);
				BushScrubPlant {
					placement,
					kind: BushScrubKind::Tuft(Arc::new(patch)),
					stick_material: MaterialRef::default(),
					ball_material: material.clone(),
					frond_material: material,
				}
			}
			BushScrubItem::Patch(patch) => {
				let variant = patch_variant_index(placed.position, patch_variants);
				let noise = variant_noise(leaf_surface_noise, variant);
				let params = stamp_foliage_noise(patch.build_tuft_patch(noise), leaf_surface_noise);
				let material = material_from_palette(
					placed.variant.palette_mix(),
					placed.position,
					leaf_surface_noise,
				);
				let (placement, patch, material) = unit_plant_from_params(
					params,
					variant,
					placed.position,
					placed.scale,
					material,
				);
				BushScrubPlant {
					placement,
					kind: BushScrubKind::Tuft(Arc::new(patch)),
					stick_material: MaterialRef::default(),
					ball_material: material.clone(),
					frond_material: material,
				}
			}
			BushScrubItem::Bush(bush) => {
				let variant = patch_variant_index(placed.position, tree_variants);
				let build_noise = variant_noise(grove_noise, variant);
				let chain_noise = variant_noise(bush_chain_noise, variant);
				let palette_noise = placement_noise(grove_noise, placed.position);
				let stick_seed = palette_noise.seed;
				let canopy_seed = palette_noise.seed.wrapping_add(31);
				let mut shape = bush.build_with_noise(build_noise);
				shape.chain_noise = chain_noise;
				let (unit_params, world_size) =
					HighBushShootsParams::new(shape).into_unit_from_num(variant);
				BushScrubPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: BushScrubKind::Bush(Arc::new(unit_params.build())),
					stick_material: stick_material_from_palette(
						Some(placed.variant.stick_palette_mix()),
						stick_seed,
					),
					ball_material: canopy_ball_material_from_palette(
						Some(placed.variant.canopy_palette_mix()),
						canopy_seed,
					),
					frond_material: frond_material_from_palette(
						Some(placed.variant.canopy_palette_mix()),
						canopy_seed,
					),
				}
			}
		}
	}

	impl VegetationComponents for BushScrub {
		fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
			Layers::new()
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
			match level {
				LodSceneLevel::High | LodSceneLevel::Medium => Layers::new(),
				LodSceneLevel::Low => {
					layers_from_nodes(foliage_low_canopy_balls(self.canopy_sites()))
				}
				LodSceneLevel::UltraLow
				| LodSceneLevel::Distance(_)
				| LodSceneLevel::Resolution(_) => layers_from_nodes(foliage_ultra_low_merged_balls(
					&self.canopy_sites(),
					ULTRA_LOW_CANOPY_BIN_METERS,
				)),
			}
		}

		fn structural_lod(&self) -> Option<StructuralLod> {
			Some(StructuralLod::new(self.structural_center, self.footprint_radius).with_factors(
				BUSH_SCRUB_STRUCTURAL_HIGH_FACTOR,
				BUSH_SCRUB_STRUCTURAL_MEDIUM_FACTOR,
				BUSH_SCRUB_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for BushScrub {
		fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
			self.structural_lod()
				.map(|band| grove_lod_level(band, lod_ref))
				.unwrap_or(LodSceneLevel::High)
		}

		fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
			self.structural_lod()
				.map(|band| grove_lod_status(band, lod_ref))
				.unwrap_or(LodSceneStatus::Unchanged)
		}

		fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
			self.structural_lod()
				.map(|band| grove_lod_culls(band, lod_ref))
				.unwrap_or(LodSceneCulls::None)
		}

		fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
			match grove_detail_level(level) {
				Some(_) => chico_vegetation_components::scene_children(Vec::new()),
				None => {
					let mut children: Vec<Box<dyn Scene>> = Vec::new();
					chico_vegetation_components::append_component_scenes(
						self,
						lod_ref,
						level,
						&mut children,
					);
					chico_vegetation_components::scene_children(children)
				}
			}
		}

		fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
			woody_grove_scene_chunks(level, lod_ref, self.nest_plant_chunks(lod_ref), self)
		}

		fn scene_bounds(&self) -> Aabb3d {
			self.structural_lod()
				.map(|p| p.footprint_aabb())
				.unwrap_or_else(|| chico_vegetation_components::vegetation_bounds(self))
		}

		fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
			lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds())
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		fn small_grove() -> BushScrub {
			BushScrubParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0)))
				.build()
		}

		fn plant_unit_size(plant: &BushScrubPlant) -> f32 {
			match &plant.kind {
				BushScrubKind::Tuft(t) => t.patch_extent_xz.max(t.shape.blade_length),
				BushScrubKind::Bush(t) => t.shape.height,
			}
		}

		fn plant_seed(plant: &BushScrubPlant) -> i32 {
			match &plant.kind {
				BushScrubKind::Tuft(t) => t.shape.seed,
				BushScrubKind::Bush(t) => t.shape.chain_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed bush-scrub plants");

			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::High).len(), 0);
			assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::High).len(), 0);
			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Medium).len(), 0);
			assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::Medium).len(), 0);

			let camera = Transform::from_translation(Vec3::new(40.0, 2.0, 40.0));
			let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
			let lod_ref = LodRef {
				entity: Entity::PLACEHOLDER,
				previous_transform: &camera,
				current_transform: &camera,
				bounds: &bounds,
			};
			let high = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::High);
			let lod::SceneChunk::SubChunks(parts) = high else {
				anyhow::bail!("High bush-scrub should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High bush-scrub plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Low).len(), 0);
			let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
			assert_eq!(low_foliage, grove.plants.len());
			assert!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len() <= low_foliage);
			match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
				lod::SceneChunk::Primitive { weight, .. } => {
					assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
				}
				lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
				_ => anyhow::bail!("Low bush-scrub should emit flattened canopy kits"),
			}
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = BushScrubParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0)));
			params.patch_variants = 4;
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed bush-scrub plants");
			for plant in grove.plants.iter() {
				assert!(
					(plant_unit_size(plant) - 1.0).abs() < 1e-4,
					"expected unit size, got {}",
					plant_unit_size(plant)
				);
			}
			let seeds: HashSet<i32> = grove.plants.iter().map(plant_seed).collect();
			assert!(seeds.len() <= 4, "expected ≤4 unique unit seeds, got {}", seeds.len());
			Ok(())
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	BushScrub, BushScrubParams, BUSH_SCRUB_STRUCTURAL_HIGH_FACTOR,
	BUSH_SCRUB_STRUCTURAL_LOW_FACTOR, BUSH_SCRUB_STRUCTURAL_MEDIUM_FACTOR,
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
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn constraint_first_fit_fallback() -> Result<()> {
		// GreenTuft (index 2) rejects steepness 0.50; first-fit falls to SmallBush (index 3),
		// which allows steepness up to 0.65.
		let prepared =
			BushScrubCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.50 };
		let outcome = prepared.select_from(
			2,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
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
