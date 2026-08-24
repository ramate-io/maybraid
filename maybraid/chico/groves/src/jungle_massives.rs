//! Jungle Massives — giant upper-canopy grove above jungle lower massives
//! ([RFC-183 §3.4.7.1], [#331](https://github.com/ramate-io/maybraid/issues/331)).
//!
//! Common 70–220 m jungle storybook and banyan skyline forms. Forest-layer attachment remains a
//! follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Dense sampled canopy-density band ([`0.20`, `0.60`]).
const DENSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.2, 0.6);
/// Dense sampled jungle-growth band ([`0.20`, `0.60`]).
const DENSE_JUNGLE_GROWTH_DENSITY: UnitRange = UnitRange::new(0.2, 0.6);
/// Dense sampled descender-density band ([`0.01`, `0.03`]).
const DENSE_DESCENDER_DENSITY: UnitRange = UnitRange::new(0.01, 0.03);

/// Authored Jungle Massives grove definition.
///
/// Cell footprint sits at the RFC midpoint (`44` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<JungleMassivesCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(44.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-44.0, 44.0),
		),
		distribution: JungleMassivesCell::distribution(),
	}
}

/// Ordered jungle-massive varietals ([RFC-183 §3.4.7.1]); the explicit `None` bucket lives only in
/// the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JungleMassivesCell {
	MassiveJungleStorybook,
	MassiveHonuBanyan,
	MassiveSopesBanyan,
}

/// Typed authored geometry for one jungle-massive varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JungleMassivesItem {
	JungleStorybook(&'static JungleMassivesJungleStorybook),
	Honu(&'static JungleMassivesBanyan),
	Sope(&'static JungleMassivesBanyan),
}

/// Authored geometry ranges for one Honu or Sope banyan form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleMassivesBanyan {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub descender_density: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one Jungle Storybook Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct JungleMassivesJungleStorybook {
	pub height: UnitRange,
	pub canopy_density: UnitRange,
	pub jungle_growth_density: UnitRange,
}

const MASSIVE_JUNGLE_STORYBOOK: JungleMassivesJungleStorybook = JungleMassivesJungleStorybook {
	height: UnitRange::new(70.0, 160.0),
	canopy_density: DENSE_CANOPY_DENSITY,
	jungle_growth_density: DENSE_JUNGLE_GROWTH_DENSITY,
};

const MASSIVE_HONU_BANYAN: JungleMassivesBanyan = JungleMassivesBanyan {
	height: UnitRange::new(70.0, 200.0),
	stalk_radius: UnitRange::new(3.0, 8.0),
	canopy_spread: UnitRange::new(25.0, 75.0),
	descender_density: DENSE_DESCENDER_DENSITY,
	canopy_density: DENSE_CANOPY_DENSITY,
};

const MASSIVE_SOPE_BANYAN: JungleMassivesBanyan = JungleMassivesBanyan {
	height: UnitRange::new(60.0, 220.0),
	stalk_radius: UnitRange::new(3.0, 9.0),
	canopy_spread: UnitRange::new(28.0, 85.0),
	descender_density: DENSE_DESCENDER_DENSITY,
	canopy_density: DENSE_CANOPY_DENSITY,
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

impl JungleMassivesCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `5.0` (RFC relative proportions); the `None` weight of `24.0` puts
	/// the placed share at `5.0 / 29.0 ≈ 0.17`, lower RFC `DENSITY_RANGE` (`0.16..0.34`).
	pub fn distribution() -> GroveDistribution<Self> {
		let jungle_storybook =
			PlacementConstraints::new(UnitRange::new(0.0, 0.50), UnitRange::new(0.0, 0.44));
		let honu = PlacementConstraints::new(UnitRange::new(0.0, 0.46), UnitRange::new(0.0, 0.38));
		let sope = PlacementConstraints::new(UnitRange::new(0.0, 0.44), UnitRange::new(0.0, 0.42));
		GroveDistribution::new(vec![
			GroveBucket::none(24.0),
			GroveBucket::placed(2.0, jungle_storybook, Self::MassiveJungleStorybook),
			GroveBucket::placed(2.0, honu, Self::MassiveHonuBanyan),
			GroveBucket::placed(1.0, sope, Self::MassiveSopesBanyan),
		])
	}

	pub fn item(self) -> JungleMassivesItem {
		match self {
			Self::MassiveJungleStorybook => {
				JungleMassivesItem::JungleStorybook(&MASSIVE_JUNGLE_STORYBOOK)
			}
			Self::MassiveHonuBanyan => JungleMassivesItem::Honu(&MASSIVE_HONU_BANYAN),
			Self::MassiveSopesBanyan => JungleMassivesItem::Sope(&MASSIVE_SOPE_BANYAN),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveJungleStorybook => JUNGLE_STORYBOOK_STICK_MIX,
			Self::MassiveHonuBanyan => HONU_STICK_MIX,
			Self::MassiveSopesBanyan => SOPE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::MassiveJungleStorybook => JUNGLE_STORYBOOK_CANOPY_MIX,
			Self::MassiveHonuBanyan => HONU_CANOPY_MIX,
			Self::MassiveSopesBanyan => SOPE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	use super::variants::jungle_massives_banyan::{HonuBanyanSamples, SopeBanyanSamples};
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::*;
	use bevy::scene::prelude::Scene;
	use chico_sbs_trees::{
		HonuBanyan, HonuBanyanParams, JungleStorybookTree, JungleStorybookTreeParams, SopesBanyan,
		SopesBanyanParams,
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

	use super::{definition, JungleMassivesCell, JungleMassivesItem};
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

	/// Typical large types ~180 m (jungle storybook / honu). `grove_bands_for_typical_height(180)`.
	pub const JUNGLE_MASSIVES_STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
	pub const JUNGLE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR: f32 = 55.0;
	pub const JUNGLE_MASSIVES_STRUCTURAL_LOW_FACTOR: f32 = 85.0;

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct JungleMassivesParams {
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
		resolved_placements: Option<Vec<GroveCellVariant<JungleMassivesCell>>>,
	}

	impl Default for JungleMassivesParams {
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

	impl JungleMassivesParams {
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

		pub fn placements(&self) -> Vec<GroveCellVariant<JungleMassivesCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.placements_on(&self.terrain)
		}

		/// Select placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn placements_on(
			&self,
			world: &impl crate::GroveWorldSample,
		) -> Vec<GroveCellVariant<JungleMassivesCell>> {
			if let Some(ref resolved) = self.resolved_placements {
				return resolved.clone();
			}
			self.grove.assemble(definition()).populate(&self.extent, world)
		}

		pub fn build(&self) -> JungleMassives {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> JungleMassives {
			JungleMassives::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	#[derive(Clone)]
	enum JungleMassivesKind {
		Honu(Arc<HonuBanyan>),
		Sope(Arc<SopesBanyan>),
		JungleStorybook(Arc<JungleStorybookTree>),
	}

	#[derive(Clone)]
	pub struct JungleMassivesPlant {
		pub placement: Placement,
		kind: JungleMassivesKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct JungleMassives {
		pub plants: Arc<[JungleMassivesPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl JungleMassives {
		pub fn from_placements(
			placements: &[GroveCellVariant<JungleMassivesCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[JungleMassivesPlant]> = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, tree_variants))
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
					JungleMassivesKind::Honu(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					JungleMassivesKind::Sope(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					JungleMassivesKind::JungleStorybook(t) => nest_flattened_plant_chunk(
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
						JungleMassivesKind::Honu(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						JungleMassivesKind::Sope(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
						JungleMassivesKind::JungleStorybook(t) => {
							canopy_proxy_site(t, plant.placement, material)
						}
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<JungleMassivesCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> JungleMassivesPlant {
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
			JungleMassivesItem::Honu(banyan) => {
				let samples =
					BuildWithNoise::<HonuBanyanSamples>::build_with_noise(banyan, build_noise);
				let mut params = HonuBanyanParams::default();
				params.geometry = samples.geometry;
				params.growth_spawn_fraction = samples.growth_spawn_fraction;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				JungleMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: JungleMassivesKind::Honu(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			JungleMassivesItem::Sope(banyan) => {
				let samples =
					BuildWithNoise::<SopeBanyanSamples>::build_with_noise(banyan, build_noise);
				let mut params = SopesBanyanParams::default();
				params.geometry = samples.geometry;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				JungleMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: JungleMassivesKind::Sope(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
			JungleMassivesItem::JungleStorybook(jungle) => {
				let samples = jungle.build_with_noise(build_noise);
				let mut params = JungleStorybookTreeParams::default();
				params.geometry = samples.geometry;
				params.growth_spawn_fraction = samples.growth_spawn_fraction;
				let (unit_params, world_size) = params.into_unit_from_num(variant);
				JungleMassivesPlant {
					placement: Placement::new(placed.position, 0.0)
						.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
					kind: JungleMassivesKind::JungleStorybook(Arc::new(unit_params.build())),
					stick_material,
					ball_material,
					frond_material,
				}
			}
		}
	}

	impl VegetationComponents for JungleMassives {
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
				JUNGLE_MASSIVES_STRUCTURAL_HIGH_FACTOR,
				JUNGLE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
				JUNGLE_MASSIVES_STRUCTURAL_LOW_FACTOR,
			))
		}
	}

	impl LodScene for JungleMassives {
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

		fn small_grove() -> JungleMassives {
			JungleMassivesParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(250.0, 1.0, 250.0)))
				.build()
		}

		fn plant_height(plant: &JungleMassivesPlant) -> f32 {
			match &plant.kind {
				JungleMassivesKind::Honu(t) => t.geometry.scale.tree_height,
				JungleMassivesKind::Sope(t) => t.geometry.scale.stalk_height,
				JungleMassivesKind::JungleStorybook(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &JungleMassivesPlant) -> i32 {
			match &plant.kind {
				JungleMassivesKind::Honu(t) => t.geometry.canopy_noise.seed,
				JungleMassivesKind::Sope(t) => t.geometry.canopy_noise.seed,
				JungleMassivesKind::JungleStorybook(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed jungle-massives plants");

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
				anyhow::bail!("High jungle-massives should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High jungle-massives plants should be SceneChunk::Lazy");
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
				anyhow::bail!("Low jungle-massives should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = JungleMassivesParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(250.0, 1.0, 250.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed jungle-massives plants");
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
	JungleMassives, JungleMassivesParams, JungleMassivesPlant,
	JUNGLE_MASSIVES_STRUCTURAL_HIGH_FACTOR, JUNGLE_MASSIVES_STRUCTURAL_LOW_FACTOR,
	JUNGLE_MASSIVES_STRUCTURAL_MEDIUM_FACTOR,
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
		let dist = JungleMassivesCell::distribution();
		assert_eq!(dist.len(), 4);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 24.0);
		assert_eq!(dist.buckets[1].item, Some(JungleMassivesCell::MassiveJungleStorybook));
		assert_eq!(dist.buckets[1].weight, 2.0);
		assert_eq!(dist.buckets[2].item, Some(JungleMassivesCell::MassiveHonuBanyan));
		assert_eq!(dist.buckets[2].weight, 2.0);
		assert_eq!(dist.buckets[3].item, Some(JungleMassivesCell::MassiveSopesBanyan));
		assert_eq!(dist.buckets[3].weight, 1.0);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = JungleMassivesCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.16..=0.34).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let JungleMassivesItem::JungleStorybook(jungle) =
			JungleMassivesCell::MassiveJungleStorybook.item()
		else {
			anyhow::bail!("expected jungle storybook item");
		};
		assert_eq!(jungle.height, UnitRange::new(70.0, 160.0));
		assert_eq!(jungle.canopy_density, DENSE_CANOPY_DENSITY);
		assert_eq!(jungle.jungle_growth_density, DENSE_JUNGLE_GROWTH_DENSITY);

		let JungleMassivesItem::Honu(honu) = JungleMassivesCell::MassiveHonuBanyan.item() else {
			anyhow::bail!("expected honu item");
		};
		assert_eq!(honu.height, UnitRange::new(70.0, 200.0));
		assert_eq!(honu.descender_density, DENSE_DESCENDER_DENSITY);

		let JungleMassivesItem::Sope(sope) = JungleMassivesCell::MassiveSopesBanyan.item() else {
			anyhow::bail!("expected sope item");
		};
		assert_eq!(sope.height, UnitRange::new(60.0, 220.0));
		assert_eq!(sope.descender_density, DENSE_DESCENDER_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_match_rfc() -> Result<()> {
		let dist = JungleMassivesCell::distribution();
		let storybook = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(JungleMassivesCell::MassiveJungleStorybook))
			.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
		assert_eq!(storybook.constraints.elevation.end, 0.50);
		assert_eq!(storybook.constraints.steepness.end, 0.44);

		let honu = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(JungleMassivesCell::MassiveHonuBanyan))
			.ok_or_else(|| anyhow::anyhow!("missing honu bucket"))?;
		assert_eq!(honu.constraints.elevation.end, 0.46);
		assert_eq!(honu.constraints.steepness.end, 0.38);

		let sope = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(JungleMassivesCell::MassiveSopesBanyan))
			.ok_or_else(|| anyhow::anyhow!("missing sope bucket"))?;
		assert_eq!(sope.constraints.elevation.end, 0.44);
		assert_eq!(sope.constraints.steepness.end, 0.42);
		Ok(())
	}

	#[test]
	fn steep_slope_rejects_honu_but_allows_storybook() -> Result<()> {
		let prepared = JungleMassivesCell::distribution().prepare(
			0.0,
			0.0,
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.40 };
		let outcome = prepared.select_from(
			8,
			Vec3::new(5.0, 0.30, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, JungleMassivesCell::MassiveHonuBanyan);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			JungleMassivesCell::MassiveJungleStorybook,
			JungleMassivesCell::MassiveHonuBanyan,
			JungleMassivesCell::MassiveSopesBanyan,
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0));
		let terrain = FlatTerrainSample::default();
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
