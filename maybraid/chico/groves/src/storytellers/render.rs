//! [`RenderItem`] for populated Storyteller's groves ([#336](https://github.com/ramate-io/maybraid/issues/336)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::{BraidOakTreeSbs, KamakuraTorchSbs, PenmarchTorchSbs, StorybookTreeSbs};
use chico_sbs_trees::braid_oak_tree::BraidOakTree;
use chico_sbs_trees::kamakura_torch::KamakuraTorch;
use chico_sbs_trees::penmarch_torch::PenmarchTorch;
use chico_sbs_trees::storybook_tree::StorybookTree;
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GroveCellVariant, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use crate::storytellers::{
	definition, StorytellersBraidOak, StorytellersCell, StorytellersItem, StorytellersStorybook,
	StorytellersTorch,
};

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Storyteller's instance.
pub type StorytellersStd = Storytellers<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Storyteller's grove preview (colorful Storybook, Braid Oak, and torch upper-canopy forms).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Storytellers<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	#[command(flatten, next_help_heading = "Grove")]
	pub grove: GroveFrontend,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

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
	pub terrain: Terrain,

	#[arg(skip)]
	resolved_placements: Option<Vec<GroveCellVariant<StorytellersCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for Storytellers<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	fn default() -> Self {
		Self {
			grove: GroveFrontend::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
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

impl<StickM, StickS, LeafM, LeafS, Terrain> Storytellers<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<StorytellersCell>>,
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

	pub fn placements(&self) -> Vec<GroveCellVariant<StorytellersCell>> {
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

const STORYTELLERS_RING_SPACING_SCALE: f32 = 1.25;
const STORYTELLERS_ANCHORS_PER_RING: u32 = 5;

fn storytellers_ring_spacing(base: f32) -> f32 {
	base * STORYTELLERS_RING_SPACING_SCALE
}

impl BuildWithNoise<BraidOakTreeSbs> for StorytellersBraidOak {
	fn build_with_noise(&self, noise: NoiseParams) -> BraidOakTreeSbs {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let canopy_density = sample_f32(&config, self.canopy_density, 2.0);
		let spread_lo = height * 0.35;
		let spread_hi = height * 0.55;
		let canopy_spread = sample_f32(&config, UnitRange::new(spread_lo, spread_hi), 3.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = BraidOakTreeSbs::default();
		geometry.apply_braid_preset();
		geometry.scale.tree_height = height;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.82, span * 1.02);
		let _ = canopy_density;
		geometry.canopy_noise = noise;
		geometry
	}
}

impl BuildWithNoise<StorybookTreeSbs> for StorytellersStorybook {
	fn build_with_noise(&self, noise: NoiseParams) -> StorybookTreeSbs {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = StorybookTreeSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.rings.spacing = storytellers_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring =
			STORYTELLERS_ANCHORS_PER_RING + (canopy_density * 2.0).round() as u32;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.82, span * 1.05);
		geometry.rings.height_range = UnitRange::new(0.58, 1.0);
		geometry.canopy_noise = noise;
		geometry
	}
}

struct TorchSamples {
	height: f32,
	stalk_radius: f32,
	span: f32,
}

fn sample_torch(torch: &StorytellersTorch, noise: NoiseParams) -> TorchSamples {
	let config = NoiseConfig::new(noise);
	let height =
		sample_f32(&config, torch.height, 1.0).max(torch.height.start.min(torch.height.end));
	let stalk_radius = sample_f32(&config, torch.stalk_radius, 1.5);
	let canopy_spread = sample_f32(&config, torch.canopy_spread, 2.0);
	let span = span_fraction(canopy_spread, height);
	TorchSamples { height, stalk_radius, span }
}

impl BuildWithNoise<PenmarchTorchSbs> for StorytellersTorch {
	fn build_with_noise(&self, noise: NoiseParams) -> PenmarchTorchSbs {
		let s = sample_torch(self, noise);
		let mut geometry = PenmarchTorchSbs::default();
		geometry.scale.tree_height = s.height;
		geometry.scale.stalk_base_radius = Some(s.stalk_radius);
		geometry.rings.spacing = storytellers_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring = STORYTELLERS_ANCHORS_PER_RING;
		geometry.projection.span_fraction_of_height = UnitRange::new(s.span * 0.88, s.span * 1.08);
		geometry.canopy_noise = noise;
		geometry
	}
}

impl BuildWithNoise<KamakuraTorchSbs> for StorytellersTorch {
	fn build_with_noise(&self, noise: NoiseParams) -> KamakuraTorchSbs {
		let s = sample_torch(self, noise);
		let mut geometry = KamakuraTorchSbs::default();
		geometry.scale.tree_height = s.height;
		geometry.scale.stalk_base_radius = Some(s.stalk_radius);
		geometry.rings.spacing = storytellers_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring = STORYTELLERS_ANCHORS_PER_RING;
		geometry.projection.span_fraction_of_height = UnitRange::new(s.span * 0.88, s.span * 1.08);
		geometry.canopy_noise = noise;
		geometry
	}
}

fn placement_transform<V>(placed: &GroveCellVariant<V>) -> Transform {
	Transform {
		translation: placed.position,
		rotation: Quat::IDENTITY,
		scale: Vec3::splat(placed.scale.max(1e-4)),
	}
}

impl<StickM, StickS, LeafM, LeafS, Terrain> RenderItem
	for Storytellers<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material + WithPalette + Default + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material + WithPalette + Default + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
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
				StorytellersItem::BraidOak(oak) => {
					let geometry = oak.build_with_noise(build_noise);
					let mut tree =
						BraidOakTree::<StickM, StickS, LeafM, LeafS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.inner_leaf_material = self.leaf_material.clone();
					tree.outer_leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.inner_leaf_surface_noise = foliage_noise;
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
				StorytellersItem::Storybook(story) => {
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
				StorytellersItem::PenmarchTorch(torch) => {
					let geometry =
						BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(torch, build_noise);
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
				StorytellersItem::KamakuraTorch(torch) => {
					let geometry =
						BuildWithNoise::<KamakuraTorchSbs>::build_with_noise(torch, build_noise);
					let mut tree = KamakuraTorch::<StickM, StickS, LeafM, LeafS>::default();
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
				StorytellersItem::TorchTree(torch) => {
					let geometry =
						BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(torch, build_noise);
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

		let StorytellersItem::BraidOak(oak) = StorytellersCell::ColorfulBraidOak.item() else {
			anyhow::bail!("expected braid oak item");
		};
		let oak_geom = oak.build_with_noise(noise);
		assert!(oak_geom.scale.tree_height >= oak.height.start.min(oak.height.end));
		assert!(oak_geom.scale.tree_height <= oak.height.start.max(oak.height.end));

		let StorytellersItem::Storybook(story) = StorytellersCell::ColorfulStorybook.item() else {
			anyhow::bail!("expected storybook item");
		};
		let story_geom = story.build_with_noise(noise);
		assert!(story_geom.scale.tree_height >= story.height.start.min(story.height.end));
		assert!(story_geom.scale.tree_height <= story.height.start.max(story.height.end));

		let StorytellersItem::PenmarchTorch(torch) =
			StorytellersCell::GoldenLanternPenmarch.item()
		else {
			anyhow::bail!("expected penmarch torch item");
		};
		let torch_geom = BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(torch, noise);
		assert!(torch_geom.scale.tree_height >= torch.height.start.min(torch.height.end));
		assert!(torch_geom.scale.tree_height <= torch.height.start.max(torch.height.end));
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			StorytellersCell::ColorfulStorybook,
			StorytellersCell::ColorfulBraidOak,
			StorytellersCell::BrightCanopyStorybook,
			StorytellersCell::PinkLanternStorybook,
			StorytellersCell::RedFestivalBraidOak,
			StorytellersCell::PurpleCrownStorybook,
			StorytellersCell::BlueMoonStorybook,
			StorytellersCell::GoldenLanternPenmarch,
			StorytellersCell::BlueFlameKamakura,
			StorytellersCell::FestivalTorchTree,
			StorytellersCell::VioletCanopyBraidOak,
			StorytellersCell::GoldLeafBraidOak,
			StorytellersCell::CopperFlameBraidOak,
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
		let placement = GroveCellVariant::new(
			StorytellersCell::ColorfulStorybook,
			Vec3::new(1.0, 0.0, 2.0),
				1.0,
		);
		let item = StorytellersStd::with_resolved_placements(
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
	fn default_weights_yield_moderate_density_placements_in_preview_grid() -> Result<()> {
		let span = 220.0;
		let grove = StorytellersStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.18..=0.38).contains(&placed_share),
			"expected storytellers fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn resolved_placements_cover_all_varietal_kinds() -> Result<()> {
		let placements = vec![
			GroveCellVariant::new(
				StorytellersCell::ColorfulBraidOak,
				Vec3::new(0.0, 0.0, 0.0),
				1.0,
			),
			GroveCellVariant::new(
				StorytellersCell::ColorfulStorybook,
				Vec3::new(4.0, 0.0, 0.0),
				1.0,
			),
			GroveCellVariant::new(
				StorytellersCell::GoldenLanternPenmarch,
				Vec3::new(8.0, 0.0, 0.0),
				1.0,
			),
		];
		let item = StorytellersStd::with_resolved_placements(
			placements.clone(),
			FlatTerrainSample::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			GroveSkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements().len(), 3);
		Ok(())
	}
}
