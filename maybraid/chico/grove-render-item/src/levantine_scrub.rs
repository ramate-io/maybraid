//! [`RenderItem`] for populated Levantine Scrub groves ([#320](https://github.com/ramate-io/maybraid/issues/320)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_vegetation_components::{spawn_vegetation_components, vegetation_bounds};
use chico_sbs_trees::braid_oak_tree::BraidOakTree;
use chico_sbs_trees::penmarch_torch::PenmarchTorchParams;
use chico_sbs_trees::rorys_head_trained::RorysHeadTrainedParams;
use chico_sbs_trees::simplemans_hedge::SimplemansHedge;
use chico_sbs_trees::vase_tree::VaseTreeParams;
use chico_tree_components::HighBushShoots;
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use chico_groves::levantine_scrub::{definition, LevantineScrubCell, LevantineScrubItem};
use chico_groves::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GroveFrontend, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};

/// Uniform terrain tuned for scrub placement constraints.
#[derive(Debug, Clone, Copy, PartialEq, Args)]
#[command(next_help_heading = "Terrain")]
pub struct ScrubFlatTerrain {
	#[arg(long, default_value_t = 0.25)]
	pub elevation: f32,
	#[arg(long, default_value_t = 0.15)]
	pub steepness: f32,
}

impl Default for ScrubFlatTerrain {
	fn default() -> Self {
		Self { elevation: 0.25, steepness: 0.15 }
	}
}

impl GroveWorldSample for ScrubFlatTerrain {
	fn elevation_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		self.steepness
	}
}

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Levantine Scrub instance.
pub type LevantineScrubStd = LevantineScrub<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	ScrubFlatTerrain,
>;

/// Levantine Scrub grove preview (Rory, Vase, High Bush, Penmarch Torch, and hedge bands).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LevantineScrub<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
		help_heading = "The noise applied to the chains of sticks in trees and bushes",
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
	resolved_placements: Option<Vec<GroveCellVariant<LevantineScrubCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for LevantineScrub<StickM, StickS, LeafM, LeafS, Terrain>
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

impl<StickM, StickS, LeafM, LeafS, Terrain> LevantineScrub<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<LevantineScrubCell>>,
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

	pub fn placements(&self) -> Vec<GroveCellVariant<LevantineScrubCell>> {
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
	for LevantineScrub<StickM, StickS, LeafM, LeafS, Terrain>
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
			let chain_noise = placement_noise(self.tree_chain_noise, placed.position);
			let build_noise = placement_noise(self.grove.noise, placed.position);
			let foliage_noise = placement_noise(self.leaf_surface_noise, placed.position);

			let entities = match placed.variant.item() {
				LevantineScrubItem::RoryHead(rory) => {
					let geometry = rory.build_with_noise(build_noise);
					let mut params = RorysHeadTrainedParams::default();
					params.geometry = geometry;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				LevantineScrubItem::VaseTree(vase) => {
					let geometry = vase.build_with_noise(build_noise);
					let mut params = VaseTreeParams::default();
					params.geometry = geometry;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				LevantineScrubItem::Bush(bush) => {
					let mut shape = bush.build_with_noise(build_noise);
					shape.chain_noise = chain_noise;
					let entities = HighBushShoots::<StickM, StickS, LeafM, LeafS>::spawn_from_shape(
						shape,
						self.stick_surface_noise,
						foliage_noise,
						self.stick_material.clone(),
						self.leaf_material.clone(),
						commands,
						cascade_chunk,
						local,
					);
					let stick_seed = chain_noise.seed as i32;
					let canopy_seed = build_noise.seed as i32 + 31;
					if let Some(palette) = placed.variant.stick_palette_mix() {
						patch_spawned_leaf_material::<StickM>(
							&entities, palette, stick_seed, commands,
						);
					}
					if let Some(palette) = placed.variant.canopy_palette_mix() {
						patch_spawned_leaf_material::<LeafM>(
							&entities,
							palette,
							canopy_seed,
							commands,
						);
					}
					entities
				}
				LevantineScrubItem::PenmarchTorch(torch) => {
					let geometry = torch.build_with_noise(build_noise);
					let mut params = PenmarchTorchParams::default();
					params.geometry = geometry;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				LevantineScrubItem::BraidOak(oak) => {
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
					let stick_seed = chain_noise.seed as i32;
					let canopy_seed = build_noise.seed as i32 + 31;
					if let Some(palette) = placed.variant.stick_palette_mix() {
						patch_spawned_leaf_material::<StickM>(
							&entities, palette, stick_seed, commands,
						);
					}
					if let Some(palette) = placed.variant.canopy_palette_mix() {
						patch_spawned_leaf_material::<LeafM>(
							&entities,
							palette,
							canopy_seed,
							commands,
						);
					}
					entities
				}
				LevantineScrubItem::Hedge(hedge) => {
					let samples = hedge.build_with_noise(build_noise);
					let band = SimplemansHedge::new(
						samples.height,
						samples.footprint_xz,
						samples.density,
						samples.seed,
						self.leaf_material.clone(),
					);
					let entities = band.spawn_render_items(commands, cascade_chunk, local);
					let leaf_seed = build_noise.seed as i32 + 17;
					if let Some(palette) = placed.variant.palette_mix() {
						patch_spawned_leaf_material::<LeafM>(
							&entities, palette, leaf_seed, commands,
						);
					}
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
	use chico_sbs_trees::simplemans_hedge::SimplemansHedgeStd;

	#[test]
	fn rory_vase_bush_torch_and_hedge_geometry_build_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		let LevantineScrubItem::RoryHead(rory) = LevantineScrubCell::DryRoryHeadTrained.item()
		else {
			anyhow::bail!("expected rory item");
		};
		let rory_geom = rory.build_with_noise(noise);
		assert!(rory_geom.scale.tree_height >= rory.height.start.min(rory.height.end));
		assert!(rory_geom.scale.tree_height <= rory.height.start.max(rory.height.end));

		let LevantineScrubItem::VaseTree(vase) = LevantineScrubCell::SmallVaseTree.item() else {
			anyhow::bail!("expected vase item");
		};
		let vase_geom = vase.build_with_noise(noise);
		assert!(vase_geom.height() >= vase.height.start.min(vase.height.end));

		let LevantineScrubItem::Bush(bush) = LevantineScrubCell::DryHighBush.item() else {
			anyhow::bail!("expected bush item");
		};
		let shape = bush.build_with_noise(noise);
		assert!(bush.shoot_count.contains(&shape.shoot_count));

		let LevantineScrubItem::PenmarchTorch(torch) =
			LevantineScrubCell::SmallPenmarchTorch.item()
		else {
			anyhow::bail!("expected torch item");
		};
		let torch_geom = torch.build_with_noise(noise);
		assert!(torch_geom.height() >= torch.height.start.min(torch.height.end));

		let LevantineScrubItem::BraidOak(oak) = LevantineScrubCell::SmallBraidOak.item() else {
			anyhow::bail!("expected braid oak item");
		};
		let oak_geom = oak.build_with_noise(noise);
		assert!(oak_geom.scale.tree_height >= oak.height.start.min(oak.height.end));
		assert!(oak_geom.scale.tree_height <= oak.height.start.max(oak.height.end));

		let LevantineScrubItem::Hedge(hedge) = LevantineScrubCell::ScrubHedge.item() else {
			anyhow::bail!("expected hedge item");
		};
		let samples = hedge.build_with_noise(noise);
		assert!(samples.height >= hedge.height.start.min(hedge.height.end));
		assert!(samples.footprint_xz >= hedge.width.start.min(hedge.width.end));
		let band = SimplemansHedgeStd::new(
			samples.height,
			samples.footprint_xz,
			samples.density,
			samples.seed,
			chico_sbs_trees::SkippedLeafMeshMaterial::<StandardMaterial>::default(),
		);
		assert_eq!(band.clump_anchors().len(), band.clump_count as usize);
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			LevantineScrubCell::DryRoryHeadTrained,
			LevantineScrubCell::SmallVaseTree,
			LevantineScrubCell::DryHighBush,
			LevantineScrubCell::SmallPenmarchTorch,
			LevantineScrubCell::RedOliveTorch,
			LevantineScrubCell::SmallBraidOak,
			LevantineScrubCell::ScrubHedge,
		] {
			match cell {
				LevantineScrubCell::ScrubHedge => {
					let palette = cell.palette_mix().expect("hedge palette");
					let mut allowed = Vec::new();
					for slot in palette.slots {
						allowed.extend(slot.start.resolve());
						allowed.extend(slot.end.resolve());
					}
					assert!(!allowed.is_empty(), "unresolved hedge tokens for {cell:?}");
				}
				_ => {
					for (palette, label) in
						[(cell.stick_palette_mix(), "stick"), (cell.canopy_palette_mix(), "canopy")]
					{
						let palette = palette.expect(label);
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
		let placement =
			GroveCellVariant::new(LevantineScrubCell::DryHighBush, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let item = LevantineScrubStd::with_resolved_placements(
			vec![placement.clone()],
			ScrubFlatTerrain::default(),
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
	fn default_weights_yield_sparse_to_moderate_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let grove = LevantineScrubStd::default()
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.18..=0.48).contains(&placed_share),
			"expected scrub fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn default_extent_includes_bush_and_tree_forms() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ * 2.0;
		let grove = LevantineScrubStd::default()
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let placements = grove.placements();
		assert!(!placements.is_empty());
		assert!(
			placements.iter().any(|p| matches!(p.variant, LevantineScrubCell::DryHighBush)),
			"expected at least one dry high bush"
		);
		assert!(
			placements.iter().any(|p| {
				matches!(
					p.variant,
					LevantineScrubCell::DryRoryHeadTrained
						| LevantineScrubCell::SmallVaseTree
						| LevantineScrubCell::SmallPenmarchTorch
						| LevantineScrubCell::RedOliveTorch
						| LevantineScrubCell::SmallBraidOak
				)
			}),
			"expected at least one tree or torch form over enlarged extent"
		);
		Ok(())
	}
}
