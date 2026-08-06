//! [`RenderItem`] for populated Tropical Undergrowth groves ([#315](https://github.com/ramate-io/maybraid/issues/315)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_vegetation_components::{spawn_vegetation_components, vegetation_bounds};
use chico_ball_components::tuft::BladeTuft;
use chico_sbs_geometry::{KamakuraTorchSbs, PenmarchTorchSbs};
use chico_sbs_trees::kamakura_torch::KamakuraTorchParams;
use chico_sbs_trees::palm_bush::PalmBush;
use chico_sbs_trees::penmarch_torch::PenmarchTorchParams;
use chico_sbs_trees::rorys_head_trained::RorysHeadTrainedParams;
use chico_sbs_trees::storybook_tree::StorybookTree;
use chico_sbs_trees::vase_tree::VaseTree;
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use chico_groves::tropical_undergrowth::{
	definition, TropicalUndergrowthCell, TropicalUndergrowthItem,
};
use chico_groves::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GroveFrontend, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
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
	resolved_placements: Option<Vec<GroveCellVariant<TropicalUndergrowthCell>>>,

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

impl<StickM, StickS, LeafM, LeafS, Terrain>
	TropicalUndergrowth<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<TropicalUndergrowthCell>>,
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

	pub fn placements(&self) -> Vec<GroveCellVariant<TropicalUndergrowthCell>> {
		if let Some(ref resolved) = self.resolved_placements {
			return resolved.clone();
		}
		self.grove.assemble(definition()).populate(&self.extent, &self.terrain)
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
	for TropicalUndergrowth<StickM, StickS, LeafM, LeafS, Terrain>
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
					let mut params = RorysHeadTrainedParams::default();
					params.geometry = geometry;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
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
				TropicalUndergrowthItem::PenmarchTorch(torch) => {
					let build_noise = placement_noise(self.grove.noise, placed.position);
					let geometry =
						BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(torch, build_noise);
					let mut params = PenmarchTorchParams::default();
					params.geometry = geometry;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				TropicalUndergrowthItem::KamakuraTorch(torch) => {
					let build_noise = placement_noise(self.grove.noise, placed.position);
					let geometry =
						BuildWithNoise::<KamakuraTorchSbs>::build_with_noise(torch, build_noise);
					let mut params = KamakuraTorchParams::default();
					params.geometry = geometry;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				TropicalUndergrowthItem::TorchTree(torch) => {
					let build_noise = placement_noise(self.grove.noise, placed.position);
					let geometry =
						BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(torch, build_noise);
					let mut params = PenmarchTorchParams::default();
					params.geometry = geometry;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
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
	use chico_groves::tropical_undergrowth::variants::tropical_undergrowth_rory_head::UNDERSTORY_RORY_ANCHORS_PER_RING;
	use chico_groves::tropical_undergrowth::variants::tropical_undergrowth_vase_tree::{
		understory_ring_spacing, UNDERSTORY_ANCHORS_PER_RING,
	};
	use chico_sbs_geometry::{RorysHeadTrainedSbs, StorybookTreeSbs, VaseTreeSbs};
	use procedural_common::UnitRange;

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
		let default_rory = RorysHeadTrainedSbs::default();
		assert_eq!(rory_geom.rings.anchors_per_ring, UNDERSTORY_RORY_ANCHORS_PER_RING);
		assert!(rory_geom.rings.anchors_per_ring < default_rory.rings.anchors_per_ring);
		assert_eq!(
			rory_geom.anchor_perturbation.vertical_offset,
			default_rory.anchor_perturbation.vertical_offset
		);

		let TropicalUndergrowthItem::VaseTree(vase) = TropicalUndergrowthCell::MiniVaseTree.item()
		else {
			anyhow::bail!("expected vase item");
		};
		let vase_geom = vase.build_with_noise(noise);
		assert!(vase_geom.height() >= vase.height.start.min(vase.height.end));
		assert!(vase_geom.height() <= vase.height.start.max(vase.height.end));
		let default_vase = VaseTreeSbs::default();
		assert_eq!(vase_geom.rings.spacing, understory_ring_spacing(default_vase.rings.spacing));
		assert_eq!(vase_geom.rings.anchors_per_ring, UNDERSTORY_ANCHORS_PER_RING);
		assert!(vase_geom.rings.spacing > default_vase.rings.spacing);
		assert_eq!(
			vase_geom.anchor_perturbation.vertical_offset,
			default_vase.anchor_perturbation.vertical_offset
		);

		let TropicalUndergrowthItem::Storybook(story) =
			TropicalUndergrowthCell::MiniSparseStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		let story_geom = story.build_with_noise(noise);
		assert!(story_geom.height() >= story.height.start.min(story.height.end));
		assert!(story_geom.height() <= story.height.start.max(story.height.end));
		let default_story = StorybookTreeSbs::default();
		assert_eq!(story_geom.rings.spacing, understory_ring_spacing(default_story.rings.spacing));
		assert_eq!(story_geom.rings.anchors_per_ring, UNDERSTORY_ANCHORS_PER_RING);
		assert_eq!(story_geom.rings.height_range, UnitRange::new(0.58, 1.0));
		assert!(story_geom.rings.spacing > default_story.rings.spacing);
		assert_eq!(
			story_geom.anchor_perturbation.vertical_offset,
			default_story.anchor_perturbation.vertical_offset
		);

		let TropicalUndergrowthItem::PenmarchTorch(penmarch) =
			TropicalUndergrowthCell::MiniPenmarchTorch.item()
		else {
			anyhow::bail!("expected penmarch torch item");
		};
		let penmarch_geom = BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(penmarch, noise);
		assert!(penmarch_geom.height() >= penmarch.height.start.min(penmarch.height.end));
		assert!(penmarch_geom.height() <= penmarch.height.start.max(penmarch.height.end));
		let default_penmarch = PenmarchTorchSbs::default();
		assert_eq!(
			penmarch_geom.rings.spacing,
			understory_ring_spacing(default_penmarch.rings.spacing)
		);
		assert_eq!(penmarch_geom.rings.anchors_per_ring, UNDERSTORY_ANCHORS_PER_RING);
		assert!(penmarch_geom.rings.spacing > default_penmarch.rings.spacing);
		assert_eq!(penmarch_geom.growth.branch_depth, default_penmarch.growth.branch_depth);
		assert_eq!(
			penmarch_geom.anchor_perturbation.vertical_offset,
			default_penmarch.anchor_perturbation.vertical_offset
		);

		let TropicalUndergrowthItem::KamakuraTorch(kamakura) =
			TropicalUndergrowthCell::MiniKamakuraTorch.item()
		else {
			anyhow::bail!("expected kamakura torch item");
		};
		let kamakura_geom = BuildWithNoise::<KamakuraTorchSbs>::build_with_noise(kamakura, noise);
		assert!(kamakura_geom.height() >= kamakura.height.start.min(kamakura.height.end));
		assert!(kamakura_geom.height() <= kamakura.height.start.max(kamakura.height.end));
		let default_kamakura = KamakuraTorchSbs::default();
		assert_eq!(
			kamakura_geom.rings.spacing,
			understory_ring_spacing(default_kamakura.rings.spacing)
		);
		assert_eq!(kamakura_geom.rings.anchors_per_ring, UNDERSTORY_ANCHORS_PER_RING);
		assert!(kamakura_geom.rings.spacing > default_kamakura.rings.spacing);
		assert_eq!(
			kamakura_geom.anchor_perturbation.vertical_offset,
			default_kamakura.anchor_perturbation.vertical_offset
		);

		let TropicalUndergrowthItem::TorchTree(torch) =
			TropicalUndergrowthCell::MiniTorchTree.item()
		else {
			anyhow::bail!("expected torch tree item");
		};
		let torch_geom = BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(torch, noise);
		assert!(torch_geom.height() >= torch.height.start.min(torch.height.end));
		assert!(torch_geom.height() <= torch.height.start.max(torch.height.end));
		assert_eq!(
			torch_geom.rings.spacing,
			understory_ring_spacing(default_penmarch.rings.spacing)
		);
		assert_eq!(torch_geom.rings.anchors_per_ring, UNDERSTORY_ANCHORS_PER_RING);
		assert_eq!(
			torch_geom.anchor_perturbation.vertical_offset,
			default_penmarch.anchor_perturbation.vertical_offset
		);
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
			TropicalUndergrowthCell::MiniPenmarchTorch,
			TropicalUndergrowthCell::MiniKamakuraTorch,
			TropicalUndergrowthCell::MiniTorchTree,
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
				| TropicalUndergrowthItem::Storybook(_)
				| TropicalUndergrowthItem::PenmarchTorch(_)
				| TropicalUndergrowthItem::KamakuraTorch(_)
				| TropicalUndergrowthItem::TorchTree(_) => {
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
		let placement = GroveCellVariant::new(
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
						| TropicalUndergrowthCell::MiniPenmarchTorch
						| TropicalUndergrowthCell::MiniKamakuraTorch
						| TropicalUndergrowthCell::MiniTorchTree
				)
			})
			.count();
		assert!(palms > 0, "expected palm placements among {} total", placements.len());
		assert!(mini_trees > 0, "expected mini-tree placements among {} total", placements.len());
		Ok(())
	}
}
