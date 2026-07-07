//! [`RenderItem`] for populated Conifer Sapling groves ([#326](https://github.com/ramate-io/maybraid/issues/326)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_trees::friends_conifer::FriendsConifer;
use chico_sbs_trees::northern_conifer::NorthernConifer;
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use chico_groves::conifer_sapling::{definition, ConiferSaplingCell, ConiferSaplingItem};
use chico_groves::{
	patch_spawned_leaf_material, placement_noise, GroveCellVariant, GroveExtent, GroveFrontend,
	GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};

/// Uniform terrain tuned for conifer sapling placement constraints (RFC elevation bands overlap).
#[derive(Debug, Clone, Copy, PartialEq, Args)]
#[command(next_help_heading = "Terrain")]
pub struct SaplingFlatTerrain {
	#[arg(long, default_value_t = 0.50)]
	pub elevation: f32,
	#[arg(long, default_value_t = 0.30)]
	pub steepness: f32,
}

impl Default for SaplingFlatTerrain {
	fn default() -> Self {
		Self { elevation: 0.50, steepness: 0.30 }
	}
}

impl GroveWorldSample for SaplingFlatTerrain {
	fn elevation_at(&self, _position: Vec3) -> f32 {
		self.elevation
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		self.steepness
	}
}

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Conifer Sapling instance.
pub type ConiferSaplingStd = ConiferSapling<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	SaplingFlatTerrain,
>;

/// Conifer Sapling grove preview (young Friend's and Northern conifers).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConiferSapling<StickM, StickS, LeafM, LeafS, Terrain = SaplingFlatTerrain>
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
	resolved_placements: Option<Vec<GroveCellVariant<ConiferSaplingCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for ConiferSapling<StickM, StickS, LeafM, LeafS, Terrain>
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

impl<StickM, StickS, LeafM, LeafS, Terrain> ConiferSapling<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<ConiferSaplingCell>>,
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

	pub fn placements(&self) -> Vec<GroveCellVariant<ConiferSaplingCell>> {
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
	for ConiferSapling<StickM, StickS, LeafM, LeafS, Terrain>
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
			let stick_seed = chain_noise.seed as i32;
			let canopy_seed = build_noise.seed as i32 + 31;

			let entities = match placed.variant.item() {
				ConiferSaplingItem::FriendsConifer(conifer) => {
					let samples = conifer.build_with_noise(build_noise);
					let mut tree = FriendsConifer::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = samples.geometry;
					tree.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
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
				ConiferSaplingItem::NorthernConifer(conifer) => {
					let samples = conifer.build_with_noise(build_noise);
					let mut tree = NorthernConifer::<StickM, StickS, LeafM, LeafS>::default();
					tree.geometry = samples.geometry;
					tree.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
					tree.splay_spawn_fraction = samples.splay_spawn_fraction;
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

		for cell in [
			ConiferSaplingCell::FriendSapling,
			ConiferSaplingCell::MossyFriendSapling,
			ConiferSaplingCell::BrightFriendSapling,
		] {
			let ConiferSaplingItem::FriendsConifer(friend) = cell.item() else {
				anyhow::bail!("expected friend sapling item for {cell:?}");
			};
			let samples = friend.build_with_noise(noise);
			assert!(
				samples.geometry.scale.stalk_height >= friend.height.start.min(friend.height.end)
			);
			assert!(
				samples.geometry.scale.stalk_height <= friend.height.start.max(friend.height.end)
			);
		}

		for cell in [
			ConiferSaplingCell::NorthernSapling,
			ConiferSaplingCell::ColdNorthernSapling,
			ConiferSaplingCell::WindsweptNorthernSapling,
		] {
			let ConiferSaplingItem::NorthernConifer(northern) = cell.item() else {
				anyhow::bail!("expected northern sapling item for {cell:?}");
			};
			let samples = northern.build_with_noise(noise);
			assert!(
				samples.geometry.liams.scale.stalk_height
					>= northern.height.start.min(northern.height.end)
			);
			assert!(
				samples.geometry.liams.scale.stalk_height
					<= northern.height.start.max(northern.height.end)
			);
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			ConiferSaplingCell::FriendSapling,
			ConiferSaplingCell::NorthernSapling,
			ConiferSaplingCell::MossyFriendSapling,
			ConiferSaplingCell::ColdNorthernSapling,
			ConiferSaplingCell::BrightFriendSapling,
			ConiferSaplingCell::WindsweptNorthernSapling,
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
	fn preview_has_visible_placements() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		let sapling = ConiferSaplingStd::default().with_extent(extent);
		let placements = sapling.placements();
		assert!(!placements.is_empty(), "expected visible conifer sapling preview");
		Ok(())
	}
}
