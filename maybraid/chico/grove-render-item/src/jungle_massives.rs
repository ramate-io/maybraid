//! [`RenderItem`] for populated Jungle Massives groves ([#331](https://github.com/ramate-io/maybraid/issues/331)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_trees::honu_banyan::HonuBanyan;
use chico_sbs_trees::jungle_storybook_tree::JungleStorybookTree;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_vegetation_components::{spawn_vegetation_components, vegetation_bounds};
use chico_sbs_trees::{
	SkippedInnerLeafMeshMaterial, SkippedOuterLeafMeshMaterial, SkippedStickMeshMaterial,
};
use chico_tree_components::{SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial};
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use chico_groves::jungle_massives::variants::jungle_massives_banyan::{
	HonuBanyanSamples, SopeBanyanSamples,
};
use chico_groves::jungle_massives::{definition, JungleMassivesCell, JungleMassivesItem};
use chico_groves::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GroveFrontend, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};

/// Honu template (material slots match playground [`RenderHonuBanyan`]).
pub type JmHonu = HonuBanyan<
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

/// Sope template (LodScene / VegetationComponents).
pub type JmSope = SopesBanyan;

/// Jungle Storybook template (material slots match playground [`RenderJungleStorybookTree`]).
pub type JmJungleStorybook = JungleStorybookTree<
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

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Jungle Massives instance.
pub type JungleMassivesStd = JungleMassives<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Jungle Massives grove preview (70–220 m jungle skyline above lower massives).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct JungleMassives<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
	pub honu_template: JmHonu,

	#[arg(skip)]
	pub sope_template: JmSope,

	#[arg(skip)]
	pub jungle_storybook_template: JmJungleStorybook,

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
	resolved_placements: Option<Vec<GroveCellVariant<JungleMassivesCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for JungleMassives<StickM, StickS, LeafM, LeafS, Terrain>
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
			honu_template: JmHonu::default(),
			sope_template: JmSope::default(),
			jungle_storybook_template: JmJungleStorybook::default(),
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

impl<StickM, StickS, LeafM, LeafS, Terrain> JungleMassives<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<JungleMassivesCell>>,
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
			honu_template: JmHonu::default(),
			sope_template: JmSope::default(),
			jungle_storybook_template: JmJungleStorybook::default(),
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

	pub fn placements(&self) -> Vec<GroveCellVariant<JungleMassivesCell>> {
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
	for JungleMassives<StickM, StickS, LeafM, LeafS, Terrain>
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
				JungleMassivesItem::Honu(banyan) => {
					let samples =
						BuildWithNoise::<HonuBanyanSamples>::build_with_noise(banyan, build_noise);
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
				JungleMassivesItem::Sope(banyan) => {
					let samples =
						BuildWithNoise::<SopeBanyanSamples>::build_with_noise(banyan, build_noise);
					let mut params = self.sope_template.clone();
					params.geometry = samples.geometry;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				JungleMassivesItem::JungleStorybook(jungle) => {
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
			};
			out.extend(entities);
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::jungle_lower_massives::JungleLowerMassivesStd;
	use anyhow::Result;

	#[test]
	fn tree_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		let JungleMassivesItem::Honu(honu) = JungleMassivesCell::MassiveHonuBanyan.item() else {
			anyhow::bail!("expected honu item");
		};
		let honu_samples = BuildWithNoise::<HonuBanyanSamples>::build_with_noise(honu, noise);
		assert!(honu_samples.geometry.scale.tree_height >= honu.height.start.min(honu.height.end));
		assert!(honu_samples.geometry.scale.tree_height <= honu.height.start.max(honu.height.end));

		let JungleMassivesItem::Sope(sope) = JungleMassivesCell::MassiveSopesBanyan.item() else {
			anyhow::bail!("expected sope item");
		};
		let sope_samples = BuildWithNoise::<SopeBanyanSamples>::build_with_noise(sope, noise);
		assert!(sope_samples.geometry.scale.stalk_height >= sope.height.start.min(sope.height.end));
		assert!(sope_samples.geometry.scale.stalk_height <= sope.height.start.max(sope.height.end));

		let JungleMassivesItem::JungleStorybook(jungle) =
			JungleMassivesCell::MassiveJungleStorybook.item()
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
	fn with_resolved_placements_skips_live_selection() -> Result<()> {
		let placement = GroveCellVariant::new(
			JungleMassivesCell::MassiveJungleStorybook,
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = JungleMassivesStd::with_resolved_placements(
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
	fn default_weights_yield_upper_canopy_placements_in_preview_grid() -> Result<()> {
		let span = 220.0;
		let grove = JungleMassivesStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.25, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.16..=0.34).contains(&placed_share),
			"expected jungle-massives fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		assert!(!placements.is_empty());
		Ok(())
	}

	#[test]
	fn upper_canopy_is_sparser_than_lower_massives_on_same_extent() -> Result<()> {
		let span = 220.0;
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.15 };
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		let upper = JungleMassivesStd::default()
			.with_terrain(terrain)
			.with_extent(extent)
			.placements()
			.len();
		let lower = JungleLowerMassivesStd::default()
			.with_terrain(terrain)
			.with_extent(extent)
			.placements()
			.len();
		assert!(
			upper < lower,
			"expected upper ({upper}) sparser than lower ({lower}) on same extent"
		);
		Ok(())
	}

	#[test]
	fn resolved_placements_cover_all_varietal_kinds() -> Result<()> {
		let placements = vec![
			GroveCellVariant::new(
				JungleMassivesCell::MassiveJungleStorybook,
				Vec3::new(0.0, 0.0, 0.0),
				1.0,
			),
			GroveCellVariant::new(
				JungleMassivesCell::MassiveHonuBanyan,
				Vec3::new(4.0, 0.0, 0.0),
				1.0,
			),
			GroveCellVariant::new(
				JungleMassivesCell::MassiveSopesBanyan,
				Vec3::new(8.0, 0.0, 0.0),
				1.0,
			),
		];
		let item = JungleMassivesStd::with_resolved_placements(
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
