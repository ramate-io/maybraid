//! Shamanhome — well-known moderate sacred lower-canopy grove
//! ([RFC-183 §3.4.6.3], [#324](https://github.com/ramate-io/maybraid/issues/324)).
//!
//! Braid Oak dominates with uncommon ritual Date Palm and Sope Banyan accents.
//! Forest-layer attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Sparse sampled descender-density band ([`0.02`, `0.04`]).
const SPARSE_DESCENDER_DENSITY: UnitRange = UnitRange::new(0.02, 0.04);
/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Sparse..moderate sampled canopy-density band.
const SPARSE_TO_MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.65);

/// Authored Shamanhome grove definition.
///
/// Cell footprint sits at the RFC midpoint (`10.5` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<ShamanhomeCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(8.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-10.5, 10.5),
		),
		distribution: ShamanhomeCell::distribution(),
	}
}

/// Ordered shamanhome varietals ([RFC-183 §3.4.6.3]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShamanhomeCell {
	ShamanBraidOak,
	RedRitualBraidOak,
	GnarledElderBraidOak,
	SilverShrineBraidOak,
	CopperBranchBraidOak,
	RitualDatePalm,
	SmallSopeBanyan,
}

/// Typed authored geometry for one shamanhome varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShamanhomeItem {
	BraidOak(&'static ShamanhomeBraidOak),
	DatePalm(&'static ShamanhomeDatePalm),
	SopeBanyan(&'static ShamanhomeBanyan),
}

/// Authored geometry ranges for one Braid Oak form (shared geometry; palette differs per cell).
#[derive(Debug, Clone, PartialEq)]
pub struct ShamanhomeBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one ritual Date Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct ShamanhomeDatePalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

/// Authored geometry ranges for one small Sope Banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct ShamanhomeBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	/// Sampled descender probability band; lower values keep descenders sparse.
	pub descender_density: UnitRange,
	/// Sampled `0.0..1.0` band mapping to sparse..moderate canopy density at render time.
	pub canopy_density: UnitRange,
}

const SHAMAN_BRAID_OAK: ShamanhomeBraidOak = ShamanhomeBraidOak {
	height: UnitRange::new(4.0, 7.0),
	canopy_spread: UnitRange::new(1.6, 3.8),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const ELDER_BRAID_OAK: ShamanhomeBraidOak = ShamanhomeBraidOak {
	height: UnitRange::new(5.0, 7.0),
	canopy_spread: UnitRange::new(2.0, 4.2),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SHRINE_BRAID_OAK: ShamanhomeBraidOak = ShamanhomeBraidOak {
	height: UnitRange::new(4.0, 6.0),
	canopy_spread: UnitRange::new(1.4, 3.2),
	canopy_density: SPARSE_TO_MODERATE_CANOPY_DENSITY,
};

const RITUAL_DATE_PALM: ShamanhomeDatePalm =
	ShamanhomeDatePalm { height: UnitRange::new(4.0, 6.0), crown_density: MODERATE_CANOPY_DENSITY };

const SMALL_SOPE_BANYAN: ShamanhomeBanyan = ShamanhomeBanyan {
	height: UnitRange::new(5.0, 7.0),
	stalk_radius: UnitRange::new(0.26, 0.38),
	canopy_spread: UnitRange::new(2.2, 4.8),
	descender_density: SPARSE_DESCENDER_DENSITY,
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const SHAMAN_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_bark", "moss_bark"),
	PaletteSlot::new("gnarled_brown", "gray_brown"),
]);

const SHAMAN_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("moss_green", "light_green"),
]);

const RED_RITUAL_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ritual_red_bark", "copper_red"),
	PaletteSlot::new("dark_bark", "moss_bark"),
]);

const RED_RITUAL_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("flower_red", "moss_green"),
]);

const GNARLED_ELDER_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("gnarled_brown", "dark_bark"),
	PaletteSlot::new("moss_bark", "wet_bark"),
]);

const GNARLED_ELDER_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("moss_green", "deep_green"),
	PaletteSlot::new("emerald_green", "fresh_green"),
]);

const SILVER_SHRINE_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("ornamental_bark", "gray_brown"),
	PaletteSlot::new("moss_bark", "dark_bark"),
]);

const SILVER_SHRINE_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("silver_green", "pale_green"),
	PaletteSlot::new("olive_green", "moss_green"),
]);

const COPPER_BRANCH_BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("copper_red", "ritual_red_bark"),
	PaletteSlot::new("gnarled_brown", "dark_bark"),
]);

const COPPER_BRANCH_BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("yellow_green", "moss_green"),
	PaletteSlot::new("deep_green", "fresh_green"),
]);

const RITUAL_DATE_PALM_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("dry_brown", "gray_brown"),
]);

const RITUAL_DATE_PALM_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "date_green"),
	PaletteSlot::new("yellow_green", "fresh_green"),
]);

const SOPE_BANYAN_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const SOPE_BANYAN_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "wet_green"),
	PaletteSlot::new("blue_green", "deep_green"),
]);

impl ShamanhomeCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.10` (RFC braid-oak, date-palm, and banyan proportions plus three
	/// authored braid-oak accents); the `None` weight of `8.0` puts the placed share at
	/// `5.10 / 13.10 ≈ 0.39`, mid RFC `DENSITY_RANGE` (`0.22..0.48`).
	pub fn distribution() -> GroveDistribution<Self> {
		let shaman_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.62), UnitRange::new(0.0, 0.40));
		let red_ritual_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.58), UnitRange::new(0.0, 0.45));
		let gnarled_elder_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.65), UnitRange::new(0.0, 0.42));
		let silver_shrine_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.55), UnitRange::new(0.0, 0.38));
		let copper_branch_braid_oak =
			PlacementConstraints::new(UnitRange::new(0.00, 0.60), UnitRange::new(0.0, 0.44));
		let ritual_date_palm =
			PlacementConstraints::new(UnitRange::new(0.0, 0.45), UnitRange::new(0.0, 0.30));
		let small_sope_banyan =
			PlacementConstraints::new(UnitRange::new(0.0, 0.55), UnitRange::new(0.0, 0.36));
		GroveDistribution::new(vec![
			GroveBucket::none(6.0),
			GroveBucket::placed(2.0, shaman_braid_oak, Self::ShamanBraidOak),
			GroveBucket::placed(0.45, red_ritual_braid_oak, Self::RedRitualBraidOak),
			GroveBucket::placed(0.55, gnarled_elder_braid_oak, Self::GnarledElderBraidOak),
			GroveBucket::placed(0.30, silver_shrine_braid_oak, Self::SilverShrineBraidOak),
			GroveBucket::placed(0.25, copper_branch_braid_oak, Self::CopperBranchBraidOak),
			GroveBucket::placed(0.75, ritual_date_palm, Self::RitualDatePalm),
			GroveBucket::placed(0.80, small_sope_banyan, Self::SmallSopeBanyan),
		])
	}

	pub fn item(self) -> ShamanhomeItem {
		match self {
			Self::ShamanBraidOak | Self::RedRitualBraidOak | Self::CopperBranchBraidOak => {
				ShamanhomeItem::BraidOak(&SHAMAN_BRAID_OAK)
			}
			Self::GnarledElderBraidOak => ShamanhomeItem::BraidOak(&ELDER_BRAID_OAK),
			Self::SilverShrineBraidOak => ShamanhomeItem::BraidOak(&SHRINE_BRAID_OAK),
			Self::RitualDatePalm => ShamanhomeItem::DatePalm(&RITUAL_DATE_PALM),
			Self::SmallSopeBanyan => ShamanhomeItem::SopeBanyan(&SMALL_SOPE_BANYAN),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShamanBraidOak => SHAMAN_BRAID_OAK_STICK_MIX,
			Self::RedRitualBraidOak => RED_RITUAL_BRAID_OAK_STICK_MIX,
			Self::GnarledElderBraidOak => GNARLED_ELDER_BRAID_OAK_STICK_MIX,
			Self::SilverShrineBraidOak => SILVER_SHRINE_BRAID_OAK_STICK_MIX,
			Self::CopperBranchBraidOak => COPPER_BRANCH_BRAID_OAK_STICK_MIX,
			Self::RitualDatePalm => RITUAL_DATE_PALM_STICK_MIX,
			Self::SmallSopeBanyan => SOPE_BANYAN_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::ShamanBraidOak => SHAMAN_BRAID_OAK_CANOPY_MIX,
			Self::RedRitualBraidOak => RED_RITUAL_BRAID_OAK_CANOPY_MIX,
			Self::GnarledElderBraidOak => GNARLED_ELDER_BRAID_OAK_CANOPY_MIX,
			Self::SilverShrineBraidOak => SILVER_SHRINE_BRAID_OAK_CANOPY_MIX,
			Self::CopperBranchBraidOak => COPPER_BRANCH_BRAID_OAK_CANOPY_MIX,
			Self::RitualDatePalm => RITUAL_DATE_PALM_CANOPY_MIX,
			Self::SmallSopeBanyan => SOPE_BANYAN_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use super::variants::shamanhome_banyan::SopeBanyanSamples;
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{BraidOakTree, DatePalm, DatePalmParams, QuantizedPlant, SopesBanyan};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, ShamanhomeCell, ShamanhomeItem};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_crown, canopy_proxy_site,
		foliage_low_canopy_balls, foliage_ultra_low_merged_balls, frond_material_from_palette,
		grove_detail_level, grove_lod_culls, grove_lod_level, grove_lod_status,
		grove_structural_footprint, layers_from_nodes, nest_flattened_plant_chunk,
		placed_palm_low_fronds, placement_noise, stick_material_from_palette,
		woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample, GroveCellVariant,
		GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ, ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const SHAMANHOME_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const SHAMANHOME_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const SHAMANHOME_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct ShamanhomeParams {
		#[command(flatten, next_help_heading = "Grove")]
		pub grove: GroveFrontend,

		#[arg(
			long,
			default_value = "0,1.0,1.0,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "The noise applied to the chains of sticks in trees",
		)]
		pub tree_chain_noise: NoiseParams,

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

		/// Number of unit-height plant archetypes (`unit_from_num(0..n)`). Caps unique
		/// merged-mesh handles for High/Medium.
		#[arg(long, default_value_t = 100)]
		pub tree_variants: u32,

		#[arg(skip)]
		resolved_placements: Option<Vec<GroveCellVariant<ShamanhomeCell>>>,
	}

	impl Default for ShamanhomeParams {
		fn default() -> Self {
			Self {
				grove: GroveFrontend::default(),
				tree_chain_noise: NoiseParams::from_scalar(0.0, 1.0, 1.0, 1),
				stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
				leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
				extent: GroveExtent::new(
					Vec3::ZERO,
					Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
				),
				terrain: FlatTerrainSample { elevation: 0.25, steepness: 0.15 },
				tree_variants: 100,
				resolved_placements: None,
			}
		}
	}

	impl ShamanhomeParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<ShamanhomeCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<ShamanhomeCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> Shamanhome {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Shamanhome {
			Shamanhome::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.stick_surface_noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	#[derive(Clone)]
	enum ShamanhomeKind {
		Oak(Arc<BraidOakTree>),
		Date(Arc<DatePalm>),
		Sope(Arc<SopesBanyan>),
	}

	#[derive(Clone)]
	pub struct ShamanhomePlant {
		pub placement: Placement,
		kind: ShamanhomeKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct Shamanhome {
		pub plants: Arc<[ShamanhomePlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl Shamanhome {
		pub fn from_placements(
			placements: &[GroveCellVariant<ShamanhomeCell>],
			grove_noise: NoiseParams,
			stick_surface_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[ShamanhomePlant]> = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, stick_surface_noise, tree_variants))
				.collect::<Vec<_>>()
				.into();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
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
					ShamanhomeKind::Oak(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					ShamanhomeKind::Date(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					ShamanhomeKind::Sope(t) => nest_flattened_plant_chunk(
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
				.filter_map(|plant| {
					let material = &plant.ball_material;
					match &plant.kind {
						ShamanhomeKind::Oak(t) => canopy_proxy_site(t, plant.placement, material),
						ShamanhomeKind::Date(t) => canopy_proxy_crown(t, plant.placement, material),
						ShamanhomeKind::Sope(t) => canopy_proxy_site(t, plant.placement, material),
					}
				})
				.collect()
		}

		fn foliage_low_nodes(&self) -> Vec<FoliageNode> {
			let mut nodes = Vec::new();
			let mut sites = Vec::new();
			for plant in self.plants.iter() {
				let material = &plant.ball_material;
				match &plant.kind {
					ShamanhomeKind::Oak(t) => {
						if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
							sites.push(site);
						}
					}
					ShamanhomeKind::Date(t) => {
						nodes.extend(placed_palm_low_fronds(
							t.as_ref(),
							plant.placement,
							&plant.stick_material,
							material,
							&plant.frond_material,
						));
					}
					ShamanhomeKind::Sope(t) => {
						if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
							sites.push(site);
						}
					}
				}
			}
			nodes.extend(foliage_low_canopy_balls(sites));
			nodes
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<ShamanhomeCell>,
		grove_noise: NoiseParams,
		_stick_surface_noise: NoiseParams,
		tree_variants: u32,
	) -> ShamanhomePlant {
		let variant = patch_variant_index(placed.position, tree_variants);
		let build_noise = variant_noise(grove_noise, variant);
		let palette_noise = placement_noise(grove_noise, placed.position);
		let stick_seed = palette_noise.seed;
		let canopy_seed = palette_noise.seed.wrapping_add(31);
		let stick_material =
			stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed);
		let ball_material = canopy_ball_material_from_palette(
			Some(placed.variant.canopy_palette_mix()),
			canopy_seed,
		);
		let frond_material =
			frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);

		match placed.variant.item() {
			ShamanhomeItem::BraidOak(oak) => {
				let world_size = oak.build_with_noise(build_noise).height();
				ShamanhomePlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: ShamanhomeKind::Oak(BraidOakTree::grow_num(variant).0),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			ShamanhomeItem::DatePalm(palm) => {
				let geometry = palm.build_with_noise(build_noise);
				let mut params = DatePalmParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				ShamanhomePlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: ShamanhomeKind::Date(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			ShamanhomeItem::SopeBanyan(banyan) => {
				let world_size =
					BuildWithNoise::<SopeBanyanSamples>::build_with_noise(banyan, build_noise)
						.geometry
						.scale
						.stalk_height;
				ShamanhomePlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: ShamanhomeKind::Sope(SopesBanyan::grow_num(variant).0),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
	}

	impl VegetationComponents for Shamanhome {
		fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
			Layers::new()
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
			match level {
				LodSceneLevel::High | LodSceneLevel::Medium => Layers::new(),
				LodSceneLevel::Low => layers_from_nodes(self.foliage_low_nodes()),
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
				SHAMANHOME_STRUCTURAL_HIGH_FACTOR,
				SHAMANHOME_STRUCTURAL_MEDIUM_FACTOR,
				SHAMANHOME_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for Shamanhome {
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

		fn small_grove() -> Shamanhome {
			ShamanhomeParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)))
				.build()
		}

		fn plant_height(plant: &ShamanhomePlant) -> f32 {
			match &plant.kind {
				ShamanhomeKind::Oak(t) => t.geometry.height(),
				ShamanhomeKind::Date(t) => t.geometry.height(),
				ShamanhomeKind::Sope(t) => t.geometry.scale.stalk_height,
			}
		}

		fn plant_seed(plant: &ShamanhomePlant) -> i32 {
			match &plant.kind {
				ShamanhomeKind::Oak(t) => t.geometry.canopy_noise.seed,
				ShamanhomeKind::Date(t) => t.geometry.trunk_noise.seed,
				ShamanhomeKind::Sope(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed shamanhome plants");

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
				anyhow::bail!("High shamanhome should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High shamanhome plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Low).len(), 0);
			let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
			let palms = grove
				.plants
				.iter()
				.filter(|p| matches!(p.kind, ShamanhomeKind::Date(_)))
				.count();
			let fronds = low_foliage.iter().filter(|n| n.geometry.is_frond_collection()).count();
			assert_eq!(fronds, palms * 5);
			assert!(!grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten().is_empty());
			match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
				lod::SceneChunk::Primitive { weight, .. } => {
					assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
				}
				lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
				_ => anyhow::bail!("Low shamanhome should emit flattened kits"),
			}
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = ShamanhomeParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed shamanhome plants");
			for plant in grove.plants.iter() {
				assert!(
					(plant_height(plant) - 1.0).abs() < 1e-4,
					"expected unit height, got {}",
					plant_height(plant)
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
	Shamanhome, ShamanhomeParams, ShamanhomePlant, SHAMANHOME_STRUCTURAL_HIGH_FACTOR,
	SHAMANHOME_STRUCTURAL_LOW_FACTOR, SHAMANHOME_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = ShamanhomeCell::distribution();
		assert_eq!(dist.len(), 8);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 6.0);
		assert_eq!(dist.buckets[1].item, Some(ShamanhomeCell::ShamanBraidOak));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(ShamanhomeCell::RedRitualBraidOak));
		assert_eq!(dist.buckets[2].weight, 0.45);
		assert_eq!(dist.buckets[3].item, Some(ShamanhomeCell::GnarledElderBraidOak));
		assert_eq!(dist.buckets[3].weight, 0.55);
		assert_eq!(dist.buckets[4].item, Some(ShamanhomeCell::SilverShrineBraidOak));
		assert_eq!(dist.buckets[4].weight, 0.30);
		assert_eq!(dist.buckets[5].item, Some(ShamanhomeCell::CopperBranchBraidOak));
		assert_eq!(dist.buckets[5].weight, 0.25);
		assert_eq!(dist.buckets[6].item, Some(ShamanhomeCell::RitualDatePalm));
		assert_eq!(dist.buckets[6].weight, 0.75);
		assert_eq!(dist.buckets[7].item, Some(ShamanhomeCell::SmallSopeBanyan));
		assert_eq!(dist.buckets[7].weight, 0.80);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = ShamanhomeCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.22..=0.48).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let ShamanhomeItem::BraidOak(oak) = ShamanhomeCell::ShamanBraidOak.item() else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.height, UnitRange::new(4.0, 7.0));
		assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

		let ShamanhomeItem::BraidOak(elder) = ShamanhomeCell::GnarledElderBraidOak.item() else {
			anyhow::bail!("expected elder braid oak item");
		};
		assert_eq!(elder.height, UnitRange::new(5.0, 7.0));
		assert_eq!(elder.canopy_spread, UnitRange::new(2.0, 4.2));

		let ShamanhomeItem::BraidOak(shrine) = ShamanhomeCell::SilverShrineBraidOak.item() else {
			anyhow::bail!("expected shrine braid oak item");
		};
		assert_eq!(shrine.height, UnitRange::new(4.0, 6.0));
		assert_eq!(shrine.canopy_density, SPARSE_TO_MODERATE_CANOPY_DENSITY);

		let ShamanhomeItem::DatePalm(palm) = ShamanhomeCell::RitualDatePalm.item() else {
			anyhow::bail!("expected date palm item");
		};
		assert_eq!(palm.height, UnitRange::new(4.0, 6.0));

		let ShamanhomeItem::SopeBanyan(banyan) = ShamanhomeCell::SmallSopeBanyan.item() else {
			anyhow::bail!("expected sope banyan item");
		};
		assert_eq!(banyan.height, UnitRange::new(5.0, 7.0));
		assert_eq!(banyan.descender_density, SPARSE_DESCENDER_DENSITY);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn red_ritual_braid_oak_accepts_steeper_slope_than_ritual_date_palm() -> Result<()> {
		let prepared =
			ShamanhomeCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.32 };
		let red_outcome = prepared.select_from(
			2,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match red_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, ShamanhomeCell::RedRitualBraidOak);
			}
			other => anyhow::bail!("expected RedRitualBraidOak on moderate slope, got {other:?}"),
		}
		let palm_outcome = prepared.select_from(
			6,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match palm_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, ShamanhomeCell::RitualDatePalm);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn high_elevation_rejects_date_palm_on_steep_slopes() -> Result<()> {
		let prepared =
			ShamanhomeCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.15 };
		let outcome = prepared.select_from(
			6,
			Vec3::new(5.0, 0.50, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, ShamanhomeCell::RitualDatePalm);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn placements_break_the_cell_grid() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.12 };
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
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.12 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
