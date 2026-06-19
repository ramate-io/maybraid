//! [`RenderItem`] for populated Forlorn Savanna groves ([#351](https://github.com/ramate-io/maybraid/issues/351)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::anchors::high_bush::{
	DEFAULT_ANCHOR_LIFT_FRACTION, DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
	DEFAULT_SEGMENT_LENGTH_FRACTION_LO, DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
	DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
};
use chico_sbs_geometry::{RorysHeadTrainedSbs, StorybookTreeSbs};
use chico_sbs_trees::rorys_head_trained::RorysHeadTrained;
use chico_sbs_trees::storybook_tree::StorybookTree;
use chico_tree_components::{HighBushFoliageStyle, HighBushShoots, HighBushShootsShape};
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::forlorn_savanna::{
	definition, ForlornSavannaCell, ForlornSavannaHighBush, ForlornSavannaItem, ForlornSavannaRory,
	ForlornSavannaStorybook,
};
use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Forlorn Savanna instance.
pub type ForlornSavannaStd = ForlornSavanna<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Forlorn Savanna grove preview (sparse dry rory, acacia bush, and storybook forms).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ForlornSavanna<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
	resolved_placements: Option<Vec<GrovePlacedCell<ForlornSavannaCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for ForlornSavanna<StickM, StickS, LeafM, LeafS, Terrain>
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

impl<StickM, StickS, LeafM, LeafS, Terrain> ForlornSavanna<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<ForlornSavannaCell>>,
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

	pub fn placements(&self) -> Vec<GrovePlacedCell<ForlornSavannaCell>> {
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

impl BuildWithNoise<RorysHeadTrainedSbs> for ForlornSavannaRory {
	fn build_with_noise(&self, noise: NoiseParams) -> RorysHeadTrainedSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.75);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);

		let mut geometry = RorysHeadTrainedSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.canopy_noise = noise;
		let span = span_fraction(canopy_spread, height);
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.95, span * 1.15);
		geometry
	}
}

impl BuildWithNoise<HighBushShootsShape> for ForlornSavannaHighBush {
	fn build_with_noise(&self, noise: NoiseParams) -> HighBushShootsShape {
		let config = NoiseConfig::new(noise);
		let sample_u32 = |range: &std::ops::RangeInclusive<u32>, salt| {
			let lo = *range.start() as usize;
			let hi = (*range.end() as usize).saturating_add(1);
			config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
		};

		let height = sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let leaf_radius = sample_f32(&config, self.leaf_radius, 2.0).max(0.01);

		HighBushShootsShape {
			height,
			anchor_lift_fraction: DEFAULT_ANCHOR_LIFT_FRACTION,
			shoot_count: sample_u32(&self.shoot_count, 3.0),
			radial_strength: sample_f32(&config, self.radial_strength, 5.0),
			vertical_bias: sample_f32(&config, self.vertical_bias, 6.0),
			branch_depth: sample_u32(&self.branch_depth, 4.0) as usize,
			segment_length_fraction_lo: DEFAULT_SEGMENT_LENGTH_FRACTION_LO,
			segment_length_fraction_hi: DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
			segment_radius_fraction_lo: DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
			segment_radius_fraction_hi: DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
			leaf_radius_fraction: leaf_radius / height,
			foliage_style: HighBushFoliageStyle::PlaneSplay,
			chain_noise: noise,
		}
	}
}

impl BuildWithNoise<StorybookTreeSbs> for ForlornSavannaStorybook {
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
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.82, span * 1.05);
		geometry.rings.height_range = UnitRange::new(0.58, 1.0);
		geometry.canopy_noise = noise;
		let _ = canopy_density;
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
	for ForlornSavanna<StickM, StickS, LeafM, LeafS, Terrain>
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
				ForlornSavannaItem::Rory(rory) => {
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
				ForlornSavannaItem::HighBush(bush) => {
					let mut shape = bush.build_with_noise(build_noise);
					shape.chain_noise = chain_noise;
					let entities = HighBushShoots::<StickM, StickS, LeafM, LeafS>::spawn_from_shape(
						shape,
						self.stick_surface_noise,
						self.leaf_surface_noise,
						self.stick_material.clone(),
						self.leaf_material.clone(),
						commands,
						cascade_chunk,
						local,
					);
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
				ForlornSavannaItem::Storybook(story) => {
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

		let ForlornSavannaItem::Rory(rory) = ForlornSavannaCell::SavannaRory.item() else {
			anyhow::bail!("expected rory item");
		};
		let rory_geom = rory.build_with_noise(noise);
		assert!(rory_geom.scale.tree_height >= rory.height.start.min(rory.height.end));
		assert!(rory_geom.scale.tree_height <= rory.height.start.max(rory.height.end));

		let ForlornSavannaItem::HighBush(bush) = ForlornSavannaCell::AcaciaHighBush.item() else {
			anyhow::bail!("expected high bush item");
		};
		let shape = bush.build_with_noise(noise);
		assert!(shape.height >= bush.height.start.min(bush.height.end));
		assert_eq!(shape.foliage_style, HighBushFoliageStyle::PlaneSplay);

		let ForlornSavannaItem::Storybook(story) = ForlornSavannaCell::RareSavannaStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		let story_geom = story.build_with_noise(noise);
		assert!(story_geom.scale.tree_height >= story.height.start.min(story.height.end));
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			ForlornSavannaCell::SavannaRory,
			ForlornSavannaCell::AcaciaHighBush,
			ForlornSavannaCell::RareSavannaStorybook,
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
			GrovePlacedCell::new(ForlornSavannaCell::SavannaRory, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = ForlornSavannaStd::with_resolved_placements(
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
	fn default_weights_yield_low_density_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let grove = ForlornSavannaStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.20 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.06..=0.20).contains(&placed_share),
			"expected forlorn-savanna fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		Ok(())
	}
}
