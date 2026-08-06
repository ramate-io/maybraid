//! [`RenderItem`] for populated Goettingen Follow groves ([#325](https://github.com/ramate-io/maybraid/issues/325)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_trees::braid_oak_tree::BraidOakTreeParams;
use chico_sbs_trees::storybook_tree::StorybookTreeParams;
use chico_vegetation_components::{spawn_vegetation_components, vegetation_bounds};
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use chico_groves::goettingen_follow::{definition, GoettingenFollowCell, GoettingenFollowItem};
use chico_groves::{
	placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GroveFrontend, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Goettingen Follow instance.
pub type GoettingenFollowStd = GoettingenFollow<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Goettingen Follow grove preview (sparse braid oaks and storybook follow-layer forms).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct GoettingenFollow<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
	resolved_placements: Option<Vec<GroveCellVariant<GoettingenFollowCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for GoettingenFollow<StickM, StickS, LeafM, LeafS, Terrain>
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

impl<StickM, StickS, LeafM, LeafS, Terrain> GoettingenFollow<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<GoettingenFollowCell>>,
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

	pub fn placements(&self) -> Vec<GroveCellVariant<GoettingenFollowCell>> {
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
	for GoettingenFollow<StickM, StickS, LeafM, LeafS, Terrain>
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
		_cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let mut out = Vec::new();
		for placed in self.placements() {
			let local = transform.mul_transform(placement_transform(&placed));
			let build_noise = placement_noise(self.grove.noise, placed.position);

			let entities = match placed.variant.item() {
				GoettingenFollowItem::BraidOak(oak) => {
					let geometry = oak.build_with_noise(build_noise);
					let mut params = BraidOakTreeParams::default();
					params.geometry = geometry;
					params.stick_surface_noise =
						placement_noise(self.stick_surface_noise, placed.position);
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				GoettingenFollowItem::Storybook(story) => {
					let geometry = story.build_with_noise(build_noise);
					let mut params = StorybookTreeParams::default();
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
	use crate::shamanhome::ShamanhomeStd;
	use anyhow::Result;

	#[test]
	fn tree_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		for cell in [
			GoettingenFollowCell::FollowBraidOak,
			GoettingenFollowCell::RedBranchBraidOak,
			GoettingenFollowCell::MossyTrailBraidOak,
			GoettingenFollowCell::ParkEdgeBraidOak,
			GoettingenFollowCell::TallFollowBraidOak,
			GoettingenFollowCell::OldGrowthFollowBraidOak,
		] {
			let GoettingenFollowItem::BraidOak(oak) = cell.item() else {
				anyhow::bail!("expected braid oak item for {cell:?}");
			};
			let oak_geom = oak.build_with_noise(noise);
			assert!(oak_geom.scale.tree_height >= oak.height.start.min(oak.height.end));
			assert!(oak_geom.scale.tree_height <= oak.height.start.max(oak.height.end));
		}

		let GoettingenFollowItem::Storybook(story) = GoettingenFollowCell::FollowStorybook.item()
		else {
			anyhow::bail!("expected storybook item");
		};
		let story_geom = story.build_with_noise(noise);
		assert!(story_geom.scale.tree_height >= story.height.start.min(story.height.end));
		assert!(story_geom.scale.tree_height <= story.height.start.max(story.height.end));
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			GoettingenFollowCell::FollowBraidOak,
			GoettingenFollowCell::RedBranchBraidOak,
			GoettingenFollowCell::MossyTrailBraidOak,
			GoettingenFollowCell::ParkEdgeBraidOak,
			GoettingenFollowCell::TallFollowBraidOak,
			GoettingenFollowCell::OldGrowthFollowBraidOak,
			GoettingenFollowCell::FollowStorybook,
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
			GoettingenFollowCell::FollowBraidOak,
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = GoettingenFollowStd::with_resolved_placements(
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
	fn default_weights_yield_sparse_placements_in_preview_grid() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let follow =
			GoettingenFollowStd::default().with_terrain(terrain).with_extent(extent.clone());
		let shamanhome = ShamanhomeStd::default().with_terrain(terrain).with_extent(extent);
		let cells = follow.placement_cells().len();
		let follow_placements = follow.placements();
		let shamanhome_placements = shamanhome.placements();
		let follow_share = follow_placements.len() as f32 / cells as f32;
		assert!(
			(0.10..=0.32).contains(&follow_share),
			"expected sparse follow fill, got {follow_share} ({}/{cells})",
			follow_placements.len()
		);
		assert!(
			follow_placements.len() < shamanhome_placements.len(),
			"expected follow ({}) sparser than shamanhome ({}) on the same extent",
			follow_placements.len(),
			shamanhome_placements.len()
		);
		assert!(!follow_placements.is_empty());
		Ok(())
	}
}
