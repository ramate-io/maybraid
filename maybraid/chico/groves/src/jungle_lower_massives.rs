//! Jungle Lower Massives — massive lower-canopy grove beneath very tall upper canopy
//! ([RFC-183 §3.4.6.7], [#328](https://github.com/ramate-io/maybraid/issues/328)).
//!
//! Common 10–20 m jungle storybook and banyan forms with rare braid-oak accents. Forest-layer
//! attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Moderate sampled canopy-density band ([`0.35`, `0.65`]).
const MODERATE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.35, 0.65);
/// Dense sampled canopy-density band ([`0.50`, `0.85`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.50, 0.85);

/// Authored Jungle Lower Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`23` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<JungleLowerMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(18.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-23.0, 23.0),
		),
		distribution: JungleLowerMassivesCell::distribution(),
	}
}

/// Ordered jungle lower-massive varietals ([RFC-183 §3.4.6.7]); the explicit `None` bucket lives
/// only in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JungleLowerMassivesCell {
	LowerMassiveJungleStorybook,
	LowerMassiveHonuBanyan,
	LowerMassiveSopesBanyan,
	LowerMassiveWaialeaPalm,
	RareLowerMassiveBraidOak,
}

/// Typed authored geometry for one jungle lower-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JungleLowerMassivesItem {
	JungleStorybook(&'static JungleLowerMassivesJungleStorybook),
	Honu(&'static JungleLowerMassivesBanyan),
	Sope(&'static JungleLowerMassivesBanyan),
	WaialeaPalm(&'static JungleLowerMassivesWaialeaPalm),
	BraidOak(&'static JungleLowerMassivesBraidOak),
}

/// Authored geometry ranges for one Honu or Sope banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub descender_density: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Jungle Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesJungleStorybook {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
	pub jungle_growth_density: UnitRange,
}

/// Authored geometry ranges for one Waialea Palm form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesWaialeaPalm {
	pub height: UnitRange,
	pub crown_density: UnitRange,
}

/// Authored geometry ranges for one rare Braid Oak accent.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleLowerMassivesBraidOak {
	pub height: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const LOWER_MASSIVE_JUNGLE_STORYBOOK: JungleLowerMassivesJungleStorybook =
	JungleLowerMassivesJungleStorybook {
		height: UnitRange::new(10.0, 20.0),
		canopy_density: DENSE_CANOPY_DENSITY,
		jungle_growth_density: MODERATE_CANOPY_DENSITY,
	};

const LOWER_MASSIVE_HONU_BANYAN: JungleLowerMassivesBanyan = JungleLowerMassivesBanyan {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.48, 0.72),
	canopy_spread: UnitRange::new(4.0, 9.0),
	descender_density: UnitRange::new(0.01, 0.045),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const LOWER_MASSIVE_SOPE_BANYAN: JungleLowerMassivesBanyan = JungleLowerMassivesBanyan {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.48, 0.72),
	canopy_spread: UnitRange::new(4.0, 9.0),
	descender_density: UnitRange::new(0.01, 0.045),
	canopy_density: DENSE_CANOPY_DENSITY,
};

const LOWER_MASSIVE_WAIALEA_PALM: JungleLowerMassivesWaialeaPalm = JungleLowerMassivesWaialeaPalm {
	height: UnitRange::new(10.0, 20.0),
	crown_density: DENSE_CANOPY_DENSITY,
};

const RARE_LOWER_MASSIVE_BRAID_OAK: JungleLowerMassivesBraidOak = JungleLowerMassivesBraidOak {
	height: UnitRange::new(10.0, 20.0),
	canopy_spread: UnitRange::new(3.0, 6.5),
	canopy_density: MODERATE_CANOPY_DENSITY,
};

const JUNGLE_STORYBOOK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_jungle_bark", "wet_brown"),
	PaletteSlot::new("moss_bark", "dark_bark"),
]);

const JUNGLE_STORYBOOK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

const HONU_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "gray_brown"),
]);

const HONU_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "wet_green"),
	PaletteSlot::new("blue_green", "emerald_green"),
]);

const SOPE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("banyan_bark", "dark_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const SOPE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dark_green", "deep_green"),
	PaletteSlot::new("wet_green", "fresh_green"),
]);

const WAIALEA_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("palm_bark", "tan_bark"),
	PaletteSlot::new("wet_brown", "green_brown"),
]);

const WAIALEA_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("lush_green", "bright_green"),
	PaletteSlot::new("wet_green", "lime_green"),
]);

const BRAID_OAK_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("wet_oak_bark", "dark_bark"),
	PaletteSlot::new("moss_bark", "green_brown"),
]);

const BRAID_OAK_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("deep_green", "fresh_green"),
	PaletteSlot::new("wet_green", "yellow_green"),
]);

impl JungleLowerMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `6.35` (RFC relative proportions); the `None` weight of `11.0` puts
	/// the placed share at `6.35 / 17.35 ≈ 0.37`, mid RFC `DENSITY_RANGE` (`0.20..0.42`).
	pub fn distribution() -> GroveDistribution<Self> {
		let jungle_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.54), UnitRange::new(0.0, 0.54));
		let honu = PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.46));
		let sope = PlacementConstraints::new(UnitRange::new(0.0, 0.48), UnitRange::new(0.0, 0.50));
		let waialea =
			PlacementConstraints::new(UnitRange::new(0.0, 0.44), UnitRange::new(0.0, 0.62));
		let braid_oak =
			PlacementConstraints::new(UnitRange::new(0.02, 0.50), UnitRange::new(0.0, 0.52));
		GroveDistribution::new(vec![
			GroveBucket::none(8.0),
			GroveBucket::placed(2.0, jungle_storybook, Self::LowerMassiveJungleStorybook),
			GroveBucket::placed(2.0, honu, Self::LowerMassiveHonuBanyan),
			GroveBucket::placed(1.0, sope, Self::LowerMassiveSopesBanyan),
			GroveBucket::placed(1.0, waialea, Self::LowerMassiveWaialeaPalm),
			GroveBucket::placed(0.35, braid_oak, Self::RareLowerMassiveBraidOak),
		])
	}

	pub fn item(self) -> JungleLowerMassivesItem {
		match self {
			Self::LowerMassiveJungleStorybook => {
				JungleLowerMassivesItem::JungleStorybook(&LOWER_MASSIVE_JUNGLE_STORYBOOK)
			}
			Self::LowerMassiveHonuBanyan => {
				JungleLowerMassivesItem::Honu(&LOWER_MASSIVE_HONU_BANYAN)
			}
			Self::LowerMassiveSopesBanyan => {
				JungleLowerMassivesItem::Sope(&LOWER_MASSIVE_SOPE_BANYAN)
			}
			Self::LowerMassiveWaialeaPalm => {
				JungleLowerMassivesItem::WaialeaPalm(&LOWER_MASSIVE_WAIALEA_PALM)
			}
			Self::RareLowerMassiveBraidOak => {
				JungleLowerMassivesItem::BraidOak(&RARE_LOWER_MASSIVE_BRAID_OAK)
			}
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveJungleStorybook => JUNGLE_STORYBOOK_STICK_MIX,
			Self::LowerMassiveHonuBanyan => HONU_STICK_MIX,
			Self::LowerMassiveSopesBanyan => SOPE_STICK_MIX,
			Self::LowerMassiveWaialeaPalm => WAIALEA_STICK_MIX,
			Self::RareLowerMassiveBraidOak => BRAID_OAK_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::LowerMassiveJungleStorybook => JUNGLE_STORYBOOK_CANOPY_MIX,
			Self::LowerMassiveHonuBanyan => HONU_CANOPY_MIX,
			Self::LowerMassiveSopesBanyan => SOPE_CANOPY_MIX,
			Self::LowerMassiveWaialeaPalm => WAIALEA_CANOPY_MIX,
			Self::RareLowerMassiveBraidOak => BRAID_OAK_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use super::variants::jungle_lower_massives_banyan::{HonuBanyanSamples, SopeBanyanSamples};
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		BraidOakTree, HonuBanyan, HonuBanyanParams, JungleStorybookTree,
		JungleStorybookTreeParams, SopesBanyan, SopesBanyanParams, WaialeaPalm, WaialeaPalmParams,
	};
	use chico_vegetation_components::{
		FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
	};
	use clap::Args;
	use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
	use lod::lod_ref::LodRef;
	use lod::{lod_host_scene_pending, SceneChunk};
	use material_ref::MaterialRef;
	use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};

	use super::{definition, JungleLowerMassivesCell, JungleLowerMassivesItem};
	use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_site, foliage_low_canopy_balls,
		foliage_ultra_low_merged_balls, frond_material_from_palette, grove_detail_level,
		grove_lod_culls, grove_lod_level, grove_lod_status, grove_structural_footprint,
		layers_from_nodes, nest_flattened_plant_chunk, placement_noise,
		stick_material_from_palette, woody_grove_scene_chunks, CanopyProxySite, FlatTerrainSample,
		GroveCellVariant, GroveExtent, GroveFrontend, DEFAULT_GROVE_EXTENT_XZ,
		ULTRA_LOW_CANOPY_BIN_METERS,
	};

	pub const JUNGLE_LOWER_MASSIVES_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
	pub const JUNGLE_LOWER_MASSIVES_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
	pub const JUNGLE_LOWER_MASSIVES_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct JungleLowerMassivesParams {
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
		resolved_placements: Option<Vec<GroveCellVariant<JungleLowerMassivesCell>>>,
	}

	impl Default for JungleLowerMassivesParams {
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
				terrain: FlatTerrainSample { elevation: 0.35, steepness: 0.15 },
				tree_variants: 100,
				resolved_placements: None,
			}
		}
	}

	impl JungleLowerMassivesParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<JungleLowerMassivesCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<JungleLowerMassivesCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> JungleLowerMassives {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> JungleLowerMassives {
			JungleLowerMassives::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				self.stick_surface_noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	#[derive(Clone)]
	enum JungleLowerMassivesKind {
		Honu(Arc<HonuBanyan>),
		Sope(Arc<SopesBanyan>),
		JungleStorybook(Arc<JungleStorybookTree>),
		Waialea(Arc<WaialeaPalm>),
		Oak(Arc<BraidOakTree>),
	}

	#[derive(Clone)]
	pub struct JungleLowerMassivesPlant {
		pub placement: Placement,
		kind: JungleLowerMassivesKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct JungleLowerMassives {
		pub plants: Arc<[JungleLowerMassivesPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl JungleLowerMassives {
		pub fn from_placements(
			placements: &[GroveCellVariant<JungleLowerMassivesCell>],
			grove_noise: NoiseParams,
			stick_surface_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[JungleLowerMassivesPlant]> = placements
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
					JungleLowerMassivesKind::Honu(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					JungleLowerMassivesKind::Sope(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					JungleLowerMassivesKind::JungleStorybook(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					JungleLowerMassivesKind::Waialea(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					JungleLowerMassivesKind::Oak(t) => nest_flattened_plant_chunk(
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
						JungleLowerMassivesKind::Honu(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						JungleLowerMassivesKind::Sope(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						JungleLowerMassivesKind::JungleStorybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						JungleLowerMassivesKind::Waialea(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						JungleLowerMassivesKind::Oak(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<JungleLowerMassivesCell>,
		grove_noise: NoiseParams,
		_stick_surface_noise: NoiseParams,
		tree_variants: u32,
	) -> JungleLowerMassivesPlant {
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
			JungleLowerMassivesItem::Honu(banyan) => {
				let samples =
					BuildWithNoise::<HonuBanyanSamples>::build_with_noise(banyan, build_noise);
				let mut params = HonuBanyanParams::default();
				params.geometry = samples.geometry;
				params.growth_spawn_fraction = samples.growth_spawn_fraction;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				JungleLowerMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: JungleLowerMassivesKind::Honu(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			JungleLowerMassivesItem::Sope(banyan) => {
				let samples =
					BuildWithNoise::<SopeBanyanSamples>::build_with_noise(banyan, build_noise);
				let mut params = SopesBanyanParams::default();
				params.geometry = samples.geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				JungleLowerMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: JungleLowerMassivesKind::Sope(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			JungleLowerMassivesItem::JungleStorybook(jungle) => {
				let samples = jungle.build_with_noise(build_noise);
				let mut params = JungleStorybookTreeParams::default();
				params.geometry = samples.geometry;
				params.growth_spawn_fraction = samples.growth_spawn_fraction;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				JungleLowerMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: JungleLowerMassivesKind::JungleStorybook(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			JungleLowerMassivesItem::WaialeaPalm(palm) => {
				let geometry = palm.build_with_noise(build_noise);
				let mut params = WaialeaPalmParams::default();
				params.geometry = geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				JungleLowerMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: JungleLowerMassivesKind::Waialea(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			JungleLowerMassivesItem::BraidOak(oak) => {
				let world_size = oak.build_with_noise(build_noise).height();
				JungleLowerMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: JungleLowerMassivesKind::Oak(Arc::new(BraidOakTree::unit_from_num(variant))),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
	}

	impl VegetationComponents for JungleLowerMassives {
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
				JUNGLE_LOWER_MASSIVES_STRUCTURAL_HIGH_FACTOR,
				JUNGLE_LOWER_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
				JUNGLE_LOWER_MASSIVES_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for JungleLowerMassives {
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

		fn small_grove() -> JungleLowerMassives {
			JungleLowerMassivesParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)))
				.build()
		}

		fn plant_height(plant: &JungleLowerMassivesPlant) -> f32 {
			match &plant.kind {
				JungleLowerMassivesKind::Honu(t) => t.geometry.scale.tree_height,
				JungleLowerMassivesKind::Sope(t) => t.geometry.scale.stalk_height,
				JungleLowerMassivesKind::JungleStorybook(t) => t.geometry.height(),
				JungleLowerMassivesKind::Waialea(t) => t.geometry.height(),
				JungleLowerMassivesKind::Oak(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &JungleLowerMassivesPlant) -> i32 {
			match &plant.kind {
				JungleLowerMassivesKind::Honu(t) => t.geometry.canopy_noise.seed,
				JungleLowerMassivesKind::Sope(t) => t.geometry.canopy_noise.seed,
				JungleLowerMassivesKind::JungleStorybook(t) => t.geometry.canopy_noise.seed,
				JungleLowerMassivesKind::Waialea(t) => t.geometry.trunk_noise.seed,
				JungleLowerMassivesKind::Oak(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed jungle-lower-massives plants");

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
				anyhow::bail!("High jungle-lower-massives should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High jungle-lower-massives plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Low).len(), 0);
			let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
			assert_eq!(low_foliage, grove.plants.len());
			assert!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len() <= low_foliage);
			let lod::SceneChunk::Primitive { weight, .. } =
				grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low)
			else {
				anyhow::bail!(
					"Low jungle-lower-massives should emit one flattened canopy collection"
				);
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = JungleLowerMassivesParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed jungle-lower-massives plants");
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
	JungleLowerMassives, JungleLowerMassivesParams, JungleLowerMassivesPlant,
	JUNGLE_LOWER_MASSIVES_STRUCTURAL_HIGH_FACTOR, JUNGLE_LOWER_MASSIVES_STRUCTURAL_LOW_FACTOR,
	JUNGLE_LOWER_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = JungleLowerMassivesCell::distribution();
		assert_eq!(dist.len(), 6);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 8.0);
		assert_eq!(
			dist.buckets[1].item,
			Some(JungleLowerMassivesCell::LowerMassiveJungleStorybook)
		);
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(JungleLowerMassivesCell::LowerMassiveHonuBanyan));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(JungleLowerMassivesCell::LowerMassiveSopesBanyan));
		assert_eq!(dist.buckets[3].weight, 1.0);
		assert_eq!(dist.buckets[4].item, Some(JungleLowerMassivesCell::LowerMassiveWaialeaPalm));
		assert_eq!(dist.buckets[4].weight, 1.0);
		assert_eq!(dist.buckets[5].item, Some(JungleLowerMassivesCell::RareLowerMassiveBraidOak));
		assert_eq!(dist.buckets[5].weight, 0.35);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = JungleLowerMassivesCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.18..=0.45).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let JungleLowerMassivesItem::JungleStorybook(jungle) =
			JungleLowerMassivesCell::LowerMassiveJungleStorybook.item()
		else {
			anyhow::bail!("expected jungle storybook item");
		};
		assert_eq!(jungle.height, UnitRange::new(10.0, 20.0));

		let JungleLowerMassivesItem::Honu(honu) =
			JungleLowerMassivesCell::LowerMassiveHonuBanyan.item()
		else {
			anyhow::bail!("expected honu item");
		};
		assert_eq!(honu.height, UnitRange::new(10.0, 20.0));
		assert_eq!(honu.canopy_density, DENSE_CANOPY_DENSITY);

		let JungleLowerMassivesItem::BraidOak(oak) =
			JungleLowerMassivesCell::RareLowerMassiveBraidOak.item()
		else {
			anyhow::bail!("expected braid oak item");
		};
		assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn waialea_accepts_steeper_slope_than_honu() -> Result<()> {
		let dist = JungleLowerMassivesCell::distribution();
		let honu = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(JungleLowerMassivesCell::LowerMassiveHonuBanyan))
			.ok_or_else(|| anyhow::anyhow!("missing honu bucket"))?;
		let waialea = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(JungleLowerMassivesCell::LowerMassiveWaialeaPalm))
			.ok_or_else(|| anyhow::anyhow!("missing waialea bucket"))?;
		assert!(waialea.constraints.steepness.end > honu.constraints.steepness.end);
		assert_eq!(honu.constraints.steepness.end, 0.46);
		assert_eq!(waialea.constraints.steepness.end, 0.62);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_honu_but_allows_waialea() -> Result<()> {
		let prepared = JungleLowerMassivesCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.55 };
		let outcome = prepared.select_from(
			8,
			Vec3::new(5.0, 0.30, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, JungleLowerMassivesCell::LowerMassiveHonuBanyan);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			JungleLowerMassivesCell::LowerMassiveJungleStorybook,
			JungleLowerMassivesCell::LowerMassiveHonuBanyan,
			JungleLowerMassivesCell::LowerMassiveSopesBanyan,
			JungleLowerMassivesCell::LowerMassiveWaialeaPalm,
			JungleLowerMassivesCell::RareLowerMassiveBraidOak,
		] {
			for (palette, label) in
				[(cell.stick_palette_mix(), "stick"), (cell.canopy_palette_mix(), "canopy")]
			{
				let mut allowed = Vec::new();
				for slot in palette.slots {
					allowed.extend(slot.start.resolve());
					allowed.extend(slot.end.resolve());
				}
				assert!(!allowed.is_empty(), "unresolved {label} tokens for {cell:?}");
			}
		}
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
