//! [`RenderItem`] for populated Leeward groves ([#339](https://github.com/ramate-io/maybraid/issues/339)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::{FriendsConiferSbs, StorybookTreeSbs};
use chico_sbs_trees::storybook_tree::StorybookTree;
use chico_sbs_trees::temperate_conifer::{TemperateConifer, TemperateConiferGeometry};
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::UsizeRange;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::leeward::{
	definition, LeewardCell, LeewardItem, LeewardStorybook, LeewardTemperateConifer,
};
use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Leeward instance.
pub type LeewardStd = Leeward<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Leeward grove preview (sheltered Temperate Conifer and rounded Storybook Tree forms).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Leeward<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
	resolved_placements: Option<Vec<GrovePlacedCell<LeewardCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for Leeward<StickM, StickS, LeafM, LeafS, Terrain>
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

impl<StickM, StickS, LeafM, LeafS, Terrain> Leeward<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<LeewardCell>>,
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

	pub fn placements(&self) -> Vec<GrovePlacedCell<LeewardCell>> {
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
	(canopy_spread / height.max(0.5)).clamp(0.25, 1.20)
}

const LEEWARD_RING_SPACING_SCALE: f32 = 1.22;
const LEEWARD_ANCHORS_PER_RING: u32 = 5;

fn leeward_ring_spacing(base: f32) -> f32 {
	base * LEEWARD_RING_SPACING_SCALE
}

impl BuildWithNoise<StorybookTreeSbs> for LeewardStorybook {
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
		geometry.rings.spacing = leeward_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring =
			LEEWARD_ANCHORS_PER_RING + (canopy_density * 2.0).round() as u32;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.82, span * 1.05);
		geometry.rings.height_range = UnitRange::new(0.58, 1.0);
		geometry.canopy_noise = noise;
		geometry
	}
}

struct TemperateConiferSamples {
	geometry: TemperateConiferGeometry,
	fronds_per_joint: UnitRange,
	frond_length_fraction: UnitRange,
	frond_spawn_fraction: f32,
	frond_world_scale: f32,
	apex_canopy_spawn_fraction: f32,
}

impl BuildWithNoise<TemperateConiferSamples> for LeewardTemperateConifer {
	fn build_with_noise(&self, noise: NoiseParams) -> TemperateConiferSamples {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let canopy_density = sample_f32(&config, self.canopy_density, 2.0);

		let mut inner = FriendsConiferSbs::default();
		inner.apply_temperate_preset();
		inner.scale.stalk_height = height;
		inner.scale.stalk_base_radius = Some((height * 0.025).clamp(0.18, 0.50));
		inner.projection.child_count_range = UsizeRange::new(1, 2);
		inner.canopy_noise = noise;

		let frond_spawn_fraction = (0.45 + canopy_density * 0.45).clamp(0.45, 0.95);
		let fronds_hi = 1.0 + (canopy_density * 1.0).round();
		let frond_len_lo = 0.030 + canopy_density * 0.010;
		let frond_len_hi = 0.045 + canopy_density * 0.030;

		TemperateConiferSamples {
			geometry: TemperateConiferGeometry { inner },
			fronds_per_joint: UnitRange::new(1.0, fronds_hi),
			frond_length_fraction: UnitRange::new(frond_len_lo, frond_len_hi),
			frond_spawn_fraction,
			frond_world_scale: 0.85 + canopy_density * 0.25,
			apex_canopy_spawn_fraction: 0.72 * (0.65 + canopy_density * 0.35),
		}
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
	for Leeward<StickM, StickS, LeafM, LeafS, Terrain>
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
				LeewardItem::Storybook(story) => {
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
				LeewardItem::TemperateConifer(temperate) => {
					let samples = temperate.build_with_noise(build_noise);
					let mut tree = TemperateConifer::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = samples.geometry;
					tree.frond_world_scale = samples.frond_world_scale;
					tree.fronds_per_joint = samples.fronds_per_joint;
					tree.frond_length_fraction = samples.frond_length_fraction;
					tree.frond_spawn_fraction = samples.frond_spawn_fraction;
					tree.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
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

		let LeewardItem::TemperateConifer(sheltered) =
			LeewardCell::ShelteredTemperateConifer.item()
		else {
			anyhow::bail!("expected sheltered temperate conifer item");
		};
		let temperate_samples = sheltered.build_with_noise(noise);
		assert!(
			temperate_samples.geometry.inner.scale.stalk_height
				>= sheltered.height.start.min(sheltered.height.end)
		);
		assert!(
			temperate_samples.geometry.inner.scale.stalk_height
				<= sheltered.height.start.max(sheltered.height.end)
		);

		let LeewardItem::Storybook(rounded) = LeewardCell::RoundedLeewardStorybook.item() else {
			anyhow::bail!("expected rounded leeward storybook item");
		};
		let story_geom = rounded.build_with_noise(noise);
		assert!(story_geom.scale.tree_height >= rounded.height.start.min(rounded.height.end));
		assert!(story_geom.scale.tree_height <= rounded.height.start.max(rounded.height.end));
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			LeewardCell::ShelteredTemperateConifer,
			LeewardCell::WindbreakTemperateConifer,
			LeewardCell::RoundedLeewardStorybook,
			LeewardCell::HighLeewardStorybook,
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
		let placement = GrovePlacedCell::new(
			LeewardCell::ShelteredTemperateConifer,
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = LeewardStd::with_resolved_placements(
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
		let grove = LeewardStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.18..=0.57).contains(&placed_share),
			"expected leeward fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn resolved_placements_cover_all_varietal_kinds() -> Result<()> {
		let placements = vec![
			GrovePlacedCell::new(
				LeewardCell::ShelteredTemperateConifer,
				Vec3::new(0.0, 0.0, 0.0),
				1.0,
			),
			GrovePlacedCell::new(
				LeewardCell::WindbreakTemperateConifer,
				Vec3::new(4.0, 0.0, 0.0),
				1.0,
			),
			GrovePlacedCell::new(
				LeewardCell::RoundedLeewardStorybook,
				Vec3::new(8.0, 0.0, 0.0),
				1.0,
			),
			GrovePlacedCell::new(LeewardCell::HighLeewardStorybook, Vec3::new(12.0, 0.0, 0.0), 1.0),
		];
		let item = LeewardStd::with_resolved_placements(
			placements.clone(),
			FlatTerrainSample::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			GroveSkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements().len(), 4);
		Ok(())
	}
}
