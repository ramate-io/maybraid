//! [`RenderItem`] for populated Tropical Undergrowth groves ([#315](https://github.com/ramate-io/maybraid/issues/315)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::{BladeTuft, BladeTuftShape};
use chico_sbs_geometry::{PalmBushSbs, RorysHeadTrainedSbs, StorybookTreeSbs, VaseTreeSbs};
use chico_sbs_trees::palm_bush::PalmBush;
use chico_sbs_trees::rorys_head_trained::RorysHeadTrained;
use chico_sbs_trees::storybook_tree::StorybookTree;
use chico_sbs_trees::vase_tree::VaseTree;
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{
	noise_params_from_scalar_str, BuildWithNoise, NoiseConfig, NoiseParams, UnitRange,
};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GrovePlacedCell, TerrainSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use crate::tropical_undergrowth::{
	definition, TropicalUndergrowthCell, TropicalUndergrowthItem, TropicalUndergrowthPalm,
	TropicalUndergrowthRoryHead, TropicalUndergrowthStorybook, TropicalUndergrowthTuft,
	TropicalUndergrowthVaseTree,
};

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Tropical Undergrowth instance.
pub type TropicalUndergrowthStd = TropicalUndergrowth<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Tropical Undergrowth grove preview (stick + leaf materials → tufts, palms, and mini SBS trees).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TropicalUndergrowth<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
		help_heading = "The noise applied to the chains of sticks in mini trees",
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
	resolved_placements: Option<Vec<GrovePlacedCell<TropicalUndergrowthCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for TropicalUndergrowth<StickM, StickS, LeafM, LeafS, Terrain>
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

impl<StickM, StickS, LeafM, LeafS, Terrain>
	TropicalUndergrowth<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: TerrainSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GrovePlacedCell<TropicalUndergrowthCell>>,
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

	pub fn placements(&self) -> Vec<GrovePlacedCell<TropicalUndergrowthCell>> {
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

fn sample_u32(config: &NoiseConfig, range: &std::ops::RangeInclusive<u32>, salt: f32) -> u32 {
	let lo = *range.start() as usize;
	let hi = (*range.end() as usize).saturating_add(1);
	config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
}

impl BuildWithNoise<BladeTuftShape> for TropicalUndergrowthTuft {
	fn build_with_noise(&self, noise: NoiseParams) -> BladeTuftShape {
		let config = NoiseConfig::new(noise);
		let blade_length = sample_f32(&config, self.height, 1.0).max(0.05);
		let blade_width = blade_length * sample_f32(&config, self.width_factor, 2.0);

		BladeTuftShape {
			blade_count: sample_u32(&config, &self.blade_count, 3.0),
			blade_length,
			blade_width,
			max_tilt_radians: sample_f32(&config, self.max_tilt_radians, 4.0).max(0.01),
			bend_segments: sample_u32(&config, &self.bend_segments, 5.0).max(1),
			seed: noise.seed,
			..BladeTuftShape::default()
		}
	}
}

impl BuildWithNoise<PalmBushSbs> for TropicalUndergrowthPalm {
	fn build_with_noise(&self, noise: NoiseParams) -> PalmBushSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.35);
		let frond_count = sample_u32(&config, &self.frond_count, 2.0);
		let frond_length = sample_f32(&config, self.frond_length, 3.0);
		let crown_spread = sample_f32(&config, self.crown_spread, 4.0);
		let frond_world_scale = (frond_length / height.max(0.5)).clamp(0.15, 1.2)
			* (crown_spread / height.max(0.5)).clamp(0.4, 1.5);

		let mut geometry = PalmBushSbs::default()
			.with_height(height)
			.with_frond_world_scale(frond_world_scale)
			.with_noise_params(noise);
		geometry.crown.fronds_per_ring = frond_count;
		geometry.crown.ring_count = 1;
		geometry
	}
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.25, 1.5)
}

/// Derive anchor count from crown arc length and authored limb spacing so mini trees do not
/// inherit full-size playground crowding.
fn anchors_per_ring_from_spacing(canopy_spread: f32, spacing: f32, min: u32, max: u32) -> u32 {
	let arc = std::f32::consts::PI * canopy_spread.max(0.2);
	(arc / spacing.max(0.08)).round().clamp(min as f32, max as f32) as u32
}

impl BuildWithNoise<RorysHeadTrainedSbs> for TropicalUndergrowthRoryHead {
	fn build_with_noise(&self, noise: NoiseParams) -> RorysHeadTrainedSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);

		let mut geometry = RorysHeadTrainedSbs::default();
		geometry.scale.tree_height = height;
		geometry.canopy_noise = noise;
		geometry.projection.span_fraction_of_height =
			UnitRange::new(canopy_spread * 0.85, canopy_spread * 1.05);
		// scale for the mini trees
		geometry.anchor_perturbation.vertical_offset = UnitRange::new(-0.1, 0.1);
		geometry
	}
}

impl BuildWithNoise<VaseTreeSbs> for TropicalUndergrowthVaseTree {
	fn build_with_noise(&self, noise: NoiseParams) -> VaseTreeSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.75);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let projection_spacing = sample_f32(&config, self.projection_spacing, 3.0);
		let ring_spacing = sample_f32(&config, self.ring_spacing, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = VaseTreeSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.75, span);
		geometry.growth.branch_depth = 2;
		geometry.growth.child_count_min = 1;
		geometry.growth.child_count_max = 2;
		geometry.rings.anchors_per_ring =
			anchors_per_ring_from_spacing(canopy_spread, projection_spacing, 3, 5);
		geometry.rings.spacing = ring_spacing;
		geometry.rings.height_range = UnitRange::new(0.55, 0.72);
		geometry.canopy_noise = noise;
		geometry
	}
}

impl BuildWithNoise<StorybookTreeSbs> for TropicalUndergrowthStorybook {
	fn build_with_noise(&self, noise: NoiseParams) -> StorybookTreeSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.9);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let projection_spacing = sample_f32(&config, self.projection_spacing, 3.0);
		let ring_spacing = sample_f32(&config, self.ring_spacing, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = StorybookTreeSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.70, span);
		geometry.growth.branch_depth = 2;
		geometry.growth.child_count_min = 1;
		geometry.growth.child_count_max = 2;
		geometry.rings.anchors_per_ring =
			anchors_per_ring_from_spacing(canopy_spread, projection_spacing, 3, 5);
		geometry.rings.spacing = ring_spacing;
		geometry.rings.height_range = UnitRange::new(0.50, 0.68);
		geometry.canopy_noise = noise;
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
	for TropicalUndergrowth<StickM, StickS, LeafM, LeafS, Terrain>
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
			let entities = match placed.variant.item() {
				TropicalUndergrowthItem::Tuft(tuft) => {
					let mut shape = tuft.build_with_noise(foliage_noise);
					shape.noise_amplitude = self.leaf_surface_noise.amplitude;
					shape.noise_frequency = self.leaf_surface_noise.frequency;
					let tuft = BladeTuft::from_shape(shape, self.leaf_material.clone());
					let entities = tuft.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.palette_mix(),
						foliage_noise.seed,
						commands,
					);
					entities
				}
				TropicalUndergrowthItem::Patch(patch) => {
					let mut item =
						patch.build_tuft_patch(foliage_noise, self.leaf_material.clone());
					item.shape.noise_amplitude = self.leaf_surface_noise.amplitude;
					item.shape.noise_frequency = self.leaf_surface_noise.frequency;
					let entities = item.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.palette_mix(),
						foliage_noise.seed,
						commands,
					);
					entities
				}
				TropicalUndergrowthItem::PalmBush(palm) => {
					let geometry = palm.build_with_noise(foliage_noise);
					let bush = PalmBush::new(geometry, self.leaf_material.clone());
					let entities = bush.spawn_render_items(commands, cascade_chunk, local);
					patch_spawned_leaf_material::<LeafM>(
						&entities,
						placed.variant.canopy_palette_mix(),
						foliage_noise.seed,
						commands,
					);
					entities
				}
				TropicalUndergrowthItem::RoryHead(rory) => {
					let build_noise = placement_noise(self.grove.noise, placed.position);
					let geometry = rory.build_with_noise(build_noise);
					let mut tree = RorysHeadTrained::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					let stick_seed =
						placement_noise(self.tree_chain_noise, placed.position).seed as i32;
					let canopy_seed = build_noise.seed as i32 + 31;
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
				TropicalUndergrowthItem::VaseTree(vase) => {
					let build_noise = placement_noise(self.grove.noise, placed.position);
					let geometry = vase.build_with_noise(build_noise);
					let mut tree =
						VaseTree::<StickM, StickS, LeafM, LeafS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.inner_leaf_material = self.leaf_material.clone();
					tree.outer_leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.inner_leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					let stick_seed =
						placement_noise(self.tree_chain_noise, placed.position).seed as i32;
					let canopy_seed = build_noise.seed as i32 + 31;
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
				TropicalUndergrowthItem::Storybook(story) => {
					let build_noise = placement_noise(self.grove.noise, placed.position);
					let geometry = story.build_with_noise(build_noise);
					let mut tree = StorybookTree::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = geometry;
					tree.stick_material = self.stick_material.clone();
					tree.leaf_material = self.leaf_material.clone();
					tree.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					tree.leaf_surface_noise = foliage_noise;
					let entities = tree.spawn_render_items(commands, cascade_chunk, local);
					let stick_seed =
						placement_noise(self.tree_chain_noise, placed.position).seed as i32;
					let canopy_seed = build_noise.seed as i32 + 31;
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
	fn tuft_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		for cell in [TropicalUndergrowthCell::BrightTuft, TropicalUndergrowthCell::DeepTuft] {
			let TropicalUndergrowthItem::Tuft(tuft) = cell.item() else {
				anyhow::bail!("expected tuft item for {cell:?}");
			};
			let shape = tuft.build_with_noise(noise);
			assert!(shape.blade_length >= tuft.height.start.min(tuft.height.end));
			assert!(shape.blade_length <= tuft.height.start.max(tuft.height.end));
		}
		Ok(())
	}

	#[test]
	fn palm_and_mini_tree_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		let TropicalUndergrowthItem::PalmBush(palm) = TropicalUndergrowthCell::SmallPalmBush.item()
		else {
			anyhow::bail!("expected palm item");
		};
		let palm_geom = palm.build_with_noise(noise);
		assert!(palm_geom.height() >= palm.height.start.min(palm.height.end));
		assert!(palm_geom.height() <= palm.height.start.max(palm.height.end));
		assert!(palm.frond_count.contains(&palm_geom.crown.fronds_per_ring));

		let TropicalUndergrowthItem::RoryHead(rory) =
			TropicalUndergrowthCell::MiniRoryHeadTrained.item()
		else {
			anyhow::bail!("expected rory item");
		};
		let rory_geom = rory.build_with_noise(noise);
		assert!(rory_geom.height() >= rory.height.start.min(rory.height.end));
		assert!(rory_geom.height() <= rory.height.start.max(rory.height.end));
		let stalk = rory_geom.scale.stalk_base_radius_or_default();
		assert!(stalk >= rory.stalk_radius.start.min(rory.stalk_radius.end));
		assert!(stalk <= rory.stalk_radius.start.max(rory.stalk_radius.end));
		assert!(rory_geom.rings.anchors_per_ring >= 3);

		let TropicalUndergrowthItem::VaseTree(vase) = TropicalUndergrowthCell::MiniVaseTree.item()
		else {
			anyhow::bail!("expected vase item");
		};
		let vase_geom = vase.build_with_noise(noise);
		assert!(vase_geom.height() >= vase.height.start.min(vase.height.end));
		assert!(vase_geom.height() <= vase.height.start.max(vase.height.end));
		assert!(vase_geom.rings.spacing >= vase.ring_spacing.start.min(vase.ring_spacing.end));
		assert!(vase_geom.rings.spacing <= vase.ring_spacing.start.max(vase.ring_spacing.end));
		assert!(vase_geom.rings.anchors_per_ring >= 3);

		let TropicalUndergrowthItem::Storybook(story) =
			TropicalUndergrowthCell::MiniSparseStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		let story_geom = story.build_with_noise(noise);
		assert!(story_geom.height() >= story.height.start.min(story.height.end));
		assert!(story_geom.height() <= story.height.start.max(story.height.end));
		assert!(story_geom.rings.spacing >= story.ring_spacing.start.min(story.ring_spacing.end));
		assert!(story_geom.rings.spacing <= story.ring_spacing.start.max(story.ring_spacing.end));
		assert!(story_geom.rings.anchors_per_ring >= 3);
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			TropicalUndergrowthCell::BrightTuft,
			TropicalUndergrowthCell::DeepTuft,
			TropicalUndergrowthCell::SmallPalmBush,
			TropicalUndergrowthCell::MiniRoryHeadTrained,
			TropicalUndergrowthCell::MiniVaseTree,
			TropicalUndergrowthCell::MiniSparseStorybook,
			TropicalUndergrowthCell::BrightTuftPatch,
			TropicalUndergrowthCell::DeepTuftPatch,
		] {
			match cell.item() {
				TropicalUndergrowthItem::Tuft(_) | TropicalUndergrowthItem::Patch(_) => {
					let palette = cell.palette_mix();
					let mut allowed = Vec::new();
					for slot in palette.slots {
						allowed.extend(slot.start.resolve());
						allowed.extend(slot.end.resolve());
					}
					assert!(!allowed.is_empty(), "unresolved palette tokens for {cell:?}");
				}
				TropicalUndergrowthItem::PalmBush(_)
				| TropicalUndergrowthItem::RoryHead(_)
				| TropicalUndergrowthItem::VaseTree(_)
				| TropicalUndergrowthItem::Storybook(_) => {
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
			}
		}
		Ok(())
	}

	#[test]
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement = GrovePlacedCell::new(
			TropicalUndergrowthCell::BrightTuft,
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = TropicalUndergrowthStd::with_resolved_placements(
			vec![placement.clone()],
			FlatTerrainSample::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			NoiseParams::default(),
			SkippedStickMeshMaterial::<ChicoStickMaterial>::default(),
			SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(item.placements(), vec![placement]);
		Ok(())
	}

	#[test]
	fn default_weights_yield_moderate_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let grove = TropicalUndergrowthStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.22..=0.58).contains(&placed_share),
			"expected moderate tropical-undergrowth fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		assert!(!placements.is_empty());
		Ok(())
	}

	#[test]
	fn default_extent_includes_palm_and_mini_tree_placements() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let grove = TropicalUndergrowthStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let placements = grove.placements();
		let palms = placements
			.iter()
			.filter(|p| matches!(p.variant, TropicalUndergrowthCell::SmallPalmBush))
			.count();
		let mini_trees = placements
			.iter()
			.filter(|p| {
				matches!(
					p.variant,
					TropicalUndergrowthCell::MiniRoryHeadTrained
						| TropicalUndergrowthCell::MiniVaseTree
						| TropicalUndergrowthCell::MiniSparseStorybook
				)
			})
			.count();
		assert!(palms > 0, "expected palm placements among {} total", placements.len());
		assert!(mini_trees > 0, "expected mini-tree placements among {} total", placements.len());
		Ok(())
	}
}
