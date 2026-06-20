//! [`RenderItem`] for populated Jungle Lower Massives groves ([#328](https://github.com/ramate-io/maybraid/issues/328)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_trees::braid_oak_tree::BraidOakTree;
use chico_sbs_trees::honu_banyan::HonuBanyan;
use chico_sbs_trees::jungle_storybook_tree::JungleStorybookTree;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_sbs_trees::waialea_palm::WaialeaPalm;
use chico_sbs_trees::{
	SkippedInnerLeafMeshMaterial, SkippedOuterLeafMeshMaterial, SkippedStickMeshMaterial,
};
use chico_tree_components::{SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial};
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::grove::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveExtent, GroveFrontend,
	GroveCellVariant, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};
use crate::jungle_lower_massives::variants::jungle_lower_massives_banyan::{HonuBanyanSamples, SopeBanyanSamples};
use crate::jungle_lower_massives::{definition, JungleLowerMassivesCell, JungleLowerMassivesItem};
use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};

/// Honu template (material slots match playground [`RenderHonuBanyan`]).
pub type JlmHonu = HonuBanyan<
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

/// Sope template (material slots match playground [`RenderSopesBanyan`]).
pub type JlmSope = SopesBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// Jungle Storybook template (material slots match playground [`RenderJungleStorybookTree`]).
pub type JlmJungleStorybook = JungleStorybookTree<
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

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Jungle Lower Massives instance.
pub type JungleLowerMassivesStd = JungleLowerMassives<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Jungle Lower Massives grove preview (10–20 m jungle massives beneath upper canopy).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct JungleLowerMassives<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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

	#[arg(skip)]
	pub honu_template: JlmHonu,

	#[arg(skip)]
	pub sope_template: JlmSope,

	#[arg(skip)]
	pub jungle_storybook_template: JlmJungleStorybook,

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
	resolved_placements: Option<Vec<GroveCellVariant<JungleLowerMassivesCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for JungleLowerMassives<StickM, StickS, LeafM, LeafS, Terrain>
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
			honu_template: JlmHonu::default(),
			sope_template: JlmSope::default(),
			jungle_storybook_template: JlmJungleStorybook::default(),
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
	JungleLowerMassives<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<JungleLowerMassivesCell>>,
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
			honu_template: JlmHonu::default(),
			sope_template: JlmSope::default(),
			jungle_storybook_template: JlmJungleStorybook::default(),
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

	pub fn placements(&self) -> Vec<GroveCellVariant<JungleLowerMassivesCell>> {
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
	for JungleLowerMassives<StickM, StickS, LeafM, LeafS, Terrain>
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
				JungleLowerMassivesItem::Honu(banyan) => {
					let samples = BuildWithNoise::<HonuBanyanSamples>::build_with_noise(banyan, build_noise);
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
				JungleLowerMassivesItem::Sope(banyan) => {
					let samples = BuildWithNoise::<SopeBanyanSamples>::build_with_noise(banyan, build_noise);
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
				JungleLowerMassivesItem::JungleStorybook(jungle) => {
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
				JungleLowerMassivesItem::WaialeaPalm(palm) => {
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
				JungleLowerMassivesItem::BraidOak(oak) => {
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

		let JungleLowerMassivesItem::Honu(honu) =
			JungleLowerMassivesCell::LowerMassiveHonuBanyan.item()
		else {
			anyhow::bail!("expected honu item");
		};
		let honu_samples = BuildWithNoise::<HonuBanyanSamples>::build_with_noise(honu, noise);
		assert!(honu_samples.geometry.scale.tree_height >= honu.height.start.min(honu.height.end));
		assert!(honu_samples.geometry.scale.tree_height <= honu.height.start.max(honu.height.end));

		let JungleLowerMassivesItem::Sope(sope) =
			JungleLowerMassivesCell::LowerMassiveSopesBanyan.item()
		else {
			anyhow::bail!("expected sope item");
		};
		let sope_samples = BuildWithNoise::<SopeBanyanSamples>::build_with_noise(sope, noise);
		assert!(sope_samples.geometry.scale.stalk_height >= sope.height.start.min(sope.height.end));
		assert!(sope_samples.geometry.scale.stalk_height <= sope.height.start.max(sope.height.end));

		let JungleLowerMassivesItem::JungleStorybook(jungle) =
			JungleLowerMassivesCell::LowerMassiveJungleStorybook.item()
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

		let JungleLowerMassivesItem::WaialeaPalm(palm) =
			JungleLowerMassivesCell::LowerMassiveWaialeaPalm.item()
		else {
			anyhow::bail!("expected waialea item");
		};
		let palm_geom = palm.build_with_noise(noise);
		assert!(palm_geom.scale.stalk_height >= palm.height.start.min(palm.height.end));
		assert!(palm_geom.scale.stalk_height <= palm.height.start.max(palm.height.end));

		let JungleLowerMassivesItem::BraidOak(oak) =
			JungleLowerMassivesCell::RareLowerMassiveBraidOak.item()
		else {
			anyhow::bail!("expected braid oak item");
		};
		let oak_geom = oak.build_with_noise(noise);
		assert!(oak_geom.scale.tree_height >= oak.height.start.min(oak.height.end));
		assert!(oak_geom.scale.tree_height <= oak.height.start.max(oak.height.end));
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
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement = GroveCellVariant::new(
			JungleLowerMassivesCell::LowerMassiveJungleStorybook,
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = JungleLowerMassivesStd::with_resolved_placements(
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
		let grove = JungleLowerMassivesStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.20..=0.42).contains(&placed_share),
			"expected jungle-lower-massives fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		assert!(!placements.is_empty());
		Ok(())
	}
}
