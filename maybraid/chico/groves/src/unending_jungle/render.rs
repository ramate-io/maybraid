//! [`RenderItem`] for populated Unending Jungle groves ([#322](https://github.com/ramate-io/maybraid/issues/322)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::{
	HonuBanyanSbs, JungleStorybookTreeSbs, PenmarchTorchSbs, RorysHeadTrainedSbs, SopesBanyanSbs,
	StorybookTreeSbs, WaialeaPalmSbs,
};
use chico_sbs_geometry::sbs::jungle_storybook_tree::{
	JUNGLE_ANCHORS_PER_RING, JUNGLE_LEAF_RADIUS_FRACTION, JUNGLE_STALK_BASE_RADIUS_FRACTION,
};
use chico_sbs_trees::honu_banyan::HonuBanyan;
use chico_sbs_trees::jungle_storybook_tree::JungleStorybookTree;
use chico_sbs_trees::penmarch_torch::PenmarchTorch;
use chico_sbs_trees::rorys_head_trained::RorysHeadTrained;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_sbs_trees::storybook_tree::StorybookTree;
use chico_sbs_trees::waialea_palm::WaialeaPalm;
use chico_sbs_trees::{
	SkippedInnerLeafMeshMaterial, SkippedOuterLeafMeshMaterial, SkippedStickMeshMaterial,
};
use chico_tree_components::{SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial};
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use crate::unending_jungle::{
	definition, UnendingJungleBanyan, UnendingJungleCell, UnendingJungleItem,
	UnendingJungleJungleStorybook, UnendingJungleRoryHead, UnendingJungleStorybook,
	UnendingJungleTorch, UnendingJungleWaialeaPalm,
};

/// Honu template for mini-banyan placements (material slots match playground [`RenderHonuBanyan`]).
pub type JungleHonu = HonuBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedInnerLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoLeafMaterial,
	SkippedOuterLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoStickMaterial,
	SkippedBodyMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedFoliageMeshMaterial<StandardMaterial>,
>;

/// Sope template for mini-banyan placements (material slots match playground [`RenderSopesBanyan`]).
pub type JungleSope = SopesBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// Jungle Storybook template (material slots match playground [`RenderJungleStorybookTree`]).
pub type JungleStorybookTemplate = JungleStorybookTree<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedInnerLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoLeafMaterial,
	SkippedOuterLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoStickMaterial,
	SkippedBodyMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedFoliageMeshMaterial<StandardMaterial>,
>;

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Unending Jungle instance.
pub type UnendingJungleStd = UnendingJungle<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Unending Jungle grove preview (banyans, storybook forms, torch, Rory, and palm accents).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct UnendingJungle<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: GroveFrontend,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(skip)]
	pub honu_template: JungleHonu,

	#[arg(skip)]
	pub sope_template: JungleSope,

	#[arg(skip)]
	pub jungle_storybook_template: JungleStorybookTemplate,

	#[arg(
		long,
		default_value = "0,1.0,1.0,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "The noise applied to the chains of sticks in trees and banyans",
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
	pub terrain: Terrain,

	#[arg(skip)]
	resolved_placements: Option<Vec<GrovePlacedCell<UnendingJungleCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for UnendingJungle<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			honu_template: JungleHonu::default(),
			sope_template: JungleSope::default(),
			jungle_storybook_template: JungleStorybookTemplate::default(),
			tree_chain_noise: NoiseParams::from_scalar(0.0, 1.0, 1.0, 1),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			extent: GroveExtent::new(
				Vec3::ZERO,
				Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
			),
			terrain: Terrain::default(),
			resolved_placements: None,
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> UnendingJungle<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<UnendingJungleCell>>,
		terrain: Terrain,
		tree_chain_noise: NoiseParams,
		stick_surface_noise: NoiseParams,
		leaf_surface_noise: NoiseParams,
		stick_material: StickS,
		leaf_material: LeafS,
	) -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material,
			leaf_material,
			honu_template: JungleHonu::default(),
			sope_template: JungleSope::default(),
			jungle_storybook_template: JungleStorybookTemplate::default(),
			tree_chain_noise,
			stick_surface_noise,
			leaf_surface_noise,
			extent: GroveExtent::new(
				Vec3::ZERO,
				Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
			),
			terrain,
			resolved_placements: Some(resolved_placements),
			__marker: PhantomData,
		}
	}

	pub fn with_extent(mut self, extent: GroveExtent) -> Self {
		self.extent = extent;
		self
	}

	pub fn with_terrain(mut self, terrain: Terrain) -> Self {
		self.terrain = terrain;
		self
	}

	pub fn cell_extent_xz(&self) -> Vec2 {
		self.grove.definition(definition()).cell_extent_xz
	}

	pub fn placement_cells(&self) -> Vec<gimme_gen::Cell> {
		self.extent.subdivide_xz(self.cell_extent_xz())
	}

	pub fn placements(&self) -> Vec<GrovePlacedCell<UnendingJungleCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
	}
}

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.35, 1.20)
}

/// Looser ring spacing than understory mini forms, but tighter than full-size trees.
const LOWER_CANOPY_RING_SPACING_SCALE: f32 = 1.25;
const LOWER_CANOPY_ANCHORS_PER_RING: u32 = 5;

fn lower_canopy_ring_spacing(base: f32) -> f32 {
	base * LOWER_CANOPY_RING_SPACING_SCALE
}

struct HonuBanyanSamples {
	geometry: HonuBanyanSbs,
	growth_spawn_fraction: f32,
}

impl BuildWithNoise<HonuBanyanSamples> for UnendingJungleBanyan {
	fn build_with_noise(&self, noise: NoiseParams) -> HonuBanyanSamples {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(3.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let descender_threshold = sample_f32(&config, self.descender_density, 3.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = HonuBanyanSbs::default();
		geometry.apply_mini_honu_preset();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_radius_fraction = (stalk_radius / height).clamp(0.05, 0.12);
		geometry.projection.length_fraction_of_height = UnitRange::new(span * 0.85, span);
		geometry.growth.descender_threshold = descender_threshold;
		geometry.canopy_noise = noise;

		HonuBanyanSamples { geometry, growth_spawn_fraction: canopy_density }
	}
}

struct SopeBanyanSamples {
	geometry: SopesBanyanSbs,
}

impl BuildWithNoise<SopeBanyanSamples> for UnendingJungleBanyan {
	fn build_with_noise(&self, noise: NoiseParams) -> SopeBanyanSamples {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(3.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let descender_threshold = sample_f32(&config, self.descender_density, 3.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = SopesBanyanSbs::default();
		geometry.scale.stalk_height = height;
		geometry.scale.canopy_height = height * 2.0;
		geometry.scale.stalk_base_radius = stalk_radius;
		geometry.projection.length_fraction_of_height =
			UnitRange::new(span * 0.05, span * 0.18);
		geometry.growth.descender_threshold = descender_threshold;
		geometry.leaf_ball_factor = 0.25 + canopy_density * 0.35;
		geometry.canopy_noise = noise;

		SopeBanyanSamples { geometry }
	}
}

impl BuildWithNoise<StorybookTreeSbs> for UnendingJungleStorybook {
	fn build_with_noise(&self, noise: NoiseParams) -> StorybookTreeSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(2.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = StorybookTreeSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.rings.spacing = lower_canopy_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring =
			LOWER_CANOPY_ANCHORS_PER_RING + (canopy_density * 2.0).round() as u32;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.82, span * 1.05);
		geometry.rings.height_range = UnitRange::new(0.58, 1.0);
		geometry.canopy_noise = noise;
		geometry
	}
}

struct JungleStorybookSamples {
	geometry: JungleStorybookTreeSbs,
	growth_spawn_fraction: f32,
}

impl BuildWithNoise<JungleStorybookSamples> for UnendingJungleJungleStorybook {
	fn build_with_noise(&self, noise: NoiseParams) -> JungleStorybookSamples {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(5.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 2.0);
		let growth_spawn_fraction = sample_f32(&config, self.jungle_growth_density, 3.0);

		let mut geometry = JungleStorybookTreeSbs::default();
		geometry.apply_jungle_preset();
		geometry.storybook.scale.tree_height = height;
		geometry.storybook.scale.stalk_base_radius =
			Some(JUNGLE_STALK_BASE_RADIUS_FRACTION * height);
		geometry.storybook.rings.anchors_per_ring =
			JUNGLE_ANCHORS_PER_RING + (canopy_density * 2.0).round() as u32;
		geometry.storybook.canopy.leaf_radius_fraction =
			JUNGLE_LEAF_RADIUS_FRACTION * (0.85 + canopy_density * 0.25);
		geometry.storybook.canopy_noise = noise;

		JungleStorybookSamples { geometry, growth_spawn_fraction }
	}
}

impl BuildWithNoise<PenmarchTorchSbs> for UnendingJungleTorch {
	fn build_with_noise(&self, noise: NoiseParams) -> PenmarchTorchSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(2.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let _canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = PenmarchTorchSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.rings.spacing = lower_canopy_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring = LOWER_CANOPY_ANCHORS_PER_RING;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.88, span * 1.08);
		geometry.canopy_noise = noise;
		geometry
	}
}

impl BuildWithNoise<RorysHeadTrainedSbs> for UnendingJungleRoryHead {
	fn build_with_noise(&self, noise: NoiseParams) -> RorysHeadTrainedSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(2.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let _canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = RorysHeadTrainedSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.canopy_noise = noise;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.95, span * 1.15);
		geometry
	}
}

impl BuildWithNoise<WaialeaPalmSbs> for UnendingJungleWaialeaPalm {
	fn build_with_noise(&self, noise: NoiseParams) -> WaialeaPalmSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(5.0);
		let crown_density = sample_f32(&config, self.crown_density, 2.0);

		let mut geometry = WaialeaPalmSbs::default();
		geometry.scale.stalk_height = height;
		geometry.crown.ring_count = 2 + (crown_density * 2.0).round() as u32;
		geometry.crown.fronds_per_ring = 7 + (crown_density * 6.0).round() as u32;
		geometry.frond_world_scale = 0.40 + crown_density * 0.30;
		geometry.trunk_noise = noise;
		geometry
	}
}

fn placement_transform<V>(placed: &GrovePlacedCell<V>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> RenderItem
	for UnendingJungle<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material + WithPalette + Default + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material + WithPalette + Default + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let mut out = Vec::new();
		for placed in self.placements() {
			let local = transform.mul_transform(placement_transform(&placed));
			let foliage_noise = placement_noise(self.leaf_surface_noise, placed.position);
			let build_noise = placement_noise(self.grove.noise, placed.position);
			let chain_noise = placement_noise(self.tree_chain_noise, placed.position);
			let stick_seed = chain_noise.seed as i32;
			let canopy_seed = build_noise.seed as i32 + 31;

			let entities = match placed.variant.item() {
				UnendingJungleItem::Honu(banyan) => {
					let samples: HonuBanyanSamples = banyan.build_with_noise(build_noise);
					let mut tree = self.honu_template.clone();
					tree.geometry = samples.geometry;
					tree.construction.growth_spawn_fraction = samples.growth_spawn_fraction;
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.inner_leaf_surface_noise = foliage_noise;
					tree.outer_leaf_surface_noise = foliage_noise;
					tree.growth_body_noise = foliage_noise;
					tree.growth_foliage_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<ChicoStickMaterial>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<ChicoLeafMaterial>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				UnendingJungleItem::Sope(banyan) => {
					let samples: SopeBanyanSamples = banyan.build_with_noise(build_noise);
					let mut tree = self.sope_template.clone();
					tree.geometry = samples.geometry;
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<ChicoStickMaterial>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<ChicoLeafMaterial>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				UnendingJungleItem::Storybook(story) => {
					let geometry = story.build_with_noise(build_noise);
					let mut tree = StorybookTree::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<StickM>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				UnendingJungleItem::JungleStorybook(jungle) => {
					let samples = jungle.build_with_noise(build_noise);
					let mut tree = self.jungle_storybook_template.clone();
					tree.geometry = samples.geometry;
					tree.construction.growth_spawn_fraction = samples.growth_spawn_fraction;
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.inner_leaf_surface_noise = foliage_noise;
					tree.outer_leaf_surface_noise = foliage_noise;
					tree.growth_body_noise = foliage_noise;
					tree.growth_foliage_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<ChicoStickMaterial>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<ChicoLeafMaterial>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				UnendingJungleItem::Torch(torch) => {
					let geometry = torch.build_with_noise(build_noise);
					let mut tree = PenmarchTorch::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<StickM>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				UnendingJungleItem::RoryHead(rory) => {
					let geometry = rory.build_with_noise(build_noise);
					let mut tree = RorysHeadTrained::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<StickM>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
				UnendingJungleItem::WaialeaPalm(palm) => {
					let geometry = palm.build_with_noise(build_noise);
					let mut tree = WaialeaPalm::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.foliage_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<StickM>(
						&entities,
						placed.variant.stick_palette_mix(),
						stick_seed,
						commands,
					);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						canopy_seed,
						commands,
					);
					entities
				}
			};
			out.extend(entities);
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn tree_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		let UnendingJungleItem::Honu(honu) = UnendingJungleCell::SmallHonuBanyan.item() else {
			anyhow::bail!("expected honu item");
		};
		let honu_samples: HonuBanyanSamples = honu.build_with_noise(noise);
		assert!(honu_samples.geometry.scale.tree_height >= honu.height.start.min(honu.height.end));
		assert!(honu_samples.geometry.scale.tree_height <= honu.height.start.max(honu.height.end));

		let UnendingJungleItem::Sope(sope) = UnendingJungleCell::SmallSopeBanyan.item() else {
			anyhow::bail!("expected sope item");
		};
		let sope_samples: SopeBanyanSamples = sope.build_with_noise(noise);
		assert!(sope_samples.geometry.scale.stalk_height >= sope.height.start.min(sope.height.end));
		assert!(sope_samples.geometry.scale.stalk_height <= sope.height.start.max(sope.height.end));

		let UnendingJungleItem::Storybook(story) = UnendingJungleCell::LowerStorybook.item() else {
			anyhow::bail!("expected storybook item");
		};
		let story_geom = story.build_with_noise(noise);
		assert!(story_geom.scale.tree_height >= story.height.start.min(story.height.end));
		assert!(story_geom.scale.tree_height <= story.height.start.max(story.height.end));

		let UnendingJungleItem::JungleStorybook(jungle) =
			UnendingJungleCell::SmallJungleStorybook.item()
		else {
			anyhow::bail!("expected jungle storybook item");
		};
		let jungle_samples = jungle.build_with_noise(noise);
		assert!(
			jungle_samples.geometry.storybook.scale.tree_height
				>= jungle.height.start.min(jungle.height.end)
		);
		assert!(
			jungle_samples.geometry.storybook.scale.tree_height
				<= jungle.height.start.max(jungle.height.end)
		);

		for cell in [UnendingJungleCell::PenmarchAccent, UnendingJungleCell::RedJungleTorch] {
			let UnendingJungleItem::Torch(torch) = cell.item() else {
				anyhow::bail!("expected torch item for {cell:?}");
			};
			let torch_geom = torch.build_with_noise(noise);
			assert!(torch_geom.scale.tree_height >= torch.height.start.min(torch.height.end));
			assert!(torch_geom.scale.tree_height <= torch.height.start.max(torch.height.end));
		}

		let UnendingJungleItem::RoryHead(rory) = UnendingJungleCell::RoryAccent.item() else {
			anyhow::bail!("expected rory item");
		};
		let rory_geom = rory.build_with_noise(noise);
		assert!(rory_geom.scale.tree_height >= rory.height.start.min(rory.height.end));
		assert!(rory_geom.scale.tree_height <= rory.height.start.max(rory.height.end));

		let UnendingJungleItem::WaialeaPalm(palm) = UnendingJungleCell::WaialeaPalmAccent.item()
		else {
			anyhow::bail!("expected waialea item");
		};
		let palm_geom = palm.build_with_noise(noise);
		assert!(palm_geom.scale.stalk_height >= palm.height.start.min(palm.height.end));
		assert!(palm_geom.scale.stalk_height <= palm.height.start.max(palm.height.end));
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			UnendingJungleCell::SmallHonuBanyan,
			UnendingJungleCell::SmallSopeBanyan,
			UnendingJungleCell::LowerStorybook,
			UnendingJungleCell::SmallJungleStorybook,
			UnendingJungleCell::PenmarchAccent,
			UnendingJungleCell::RedJungleTorch,
			UnendingJungleCell::RoryAccent,
			UnendingJungleCell::WaialeaPalmAccent,
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
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement =
			GrovePlacedCell::new(UnendingJungleCell::LowerStorybook, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = UnendingJungleStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			GroveSkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn default_weights_yield_moderate_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let grove = UnendingJungleStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.24..=0.52).contains(&placed_share),
			"expected unending-jungle fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		assert!(!placements.is_empty());
		Ok(())
	}
}
