//! [`RenderItem`] for populated Conifer Massives groves ([#343](https://github.com/ramate-io/maybraid/issues/343)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_trees::friends_conifer::FriendsConiferParams;
use chico_sbs_trees::liams_conifer::LiamsConiferParams;
use chico_sbs_trees::northern_conifer::NorthernConiferParams;
use chico_sbs_trees::temperate_conifer::TemperateConiferParams;
use chico_vegetation_components::{spawn_vegetation_components, vegetation_bounds};
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use chico_groves::conifer_massives::{definition, ConiferMassivesCell, ConiferMassivesItem};
use chico_groves::{
	placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GroveFrontend, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Conifer Massives instance.
pub type ConiferMassivesStd = ConiferMassives<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Conifer Massives grove preview (giant evergreen skyline forms).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConiferMassives<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
	resolved_placements: Option<Vec<GroveCellVariant<ConiferMassivesCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for ConiferMassives<StickM, StickS, LeafM, LeafS, Terrain>
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

impl<StickM, StickS, LeafM, LeafS, Terrain> ConiferMassives<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<ConiferMassivesCell>>,
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

	pub fn placements(&self) -> Vec<GroveCellVariant<ConiferMassivesCell>> {
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
	for ConiferMassives<StickM, StickS, LeafM, LeafS, Terrain>
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
				ConiferMassivesItem::NorthernConifer(conifer) => {
					let samples = conifer.build_with_noise(build_noise);
					let mut params = NorthernConiferParams::default();
					params.geometry = samples.geometry;
					params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
					params.splay_spawn_fraction = samples.splay_spawn_fraction;
					params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				ConiferMassivesItem::FriendsConifer(conifer) => {
					let samples = conifer.build_with_noise(build_noise);
					let mut params = FriendsConiferParams::default();
					params.geometry = samples.geometry;
					params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
					params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				ConiferMassivesItem::LiamsConifer(conifer) => {
					let geometry = conifer.build_with_noise(build_noise);
					let mut params = LiamsConiferParams::default();
					params.geometry = geometry;
					let tree = params.build();
					let bounds = vegetation_bounds(&tree);
					spawn_vegetation_components(commands, &tree, local, bounds)
				}
				ConiferMassivesItem::TemperateConifer(temperate) => {
					let samples = temperate.build_with_noise(build_noise);
					let mut params = TemperateConiferParams::default();
					params.geometry = samples.geometry;
					params.frond_world_scale = samples.frond_world_scale;
					params.fronds_per_joint = samples.fronds_per_joint;
					params.frond_length_fraction = samples.frond_length_fraction;
					params.frond_spawn_fraction = samples.frond_spawn_fraction;
					params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
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

	#[test]
	fn tree_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		let ConiferMassivesItem::NorthernConifer(northern) =
			ConiferMassivesCell::MassiveNorthernConifer.item()
		else {
			anyhow::bail!("expected northern conifer item");
		};
		let northern_samples = northern.build_with_noise(noise);
		assert!(
			northern_samples.geometry.liams.scale.stalk_height
				>= northern.height.start.min(northern.height.end)
		);
		assert!(
			northern_samples.geometry.liams.scale.stalk_height
				<= northern.height.start.max(northern.height.end)
		);

		let ConiferMassivesItem::FriendsConifer(friends) =
			ConiferMassivesCell::MassiveFriendsConifer.item()
		else {
			anyhow::bail!("expected friends conifer item");
		};
		let friends_samples = friends.build_with_noise(noise);
		assert!(
			friends_samples.geometry.scale.stalk_height
				>= friends.height.start.min(friends.height.end)
		);
		assert!(
			friends_samples.geometry.scale.stalk_height
				<= friends.height.start.max(friends.height.end)
		);

		let ConiferMassivesItem::TemperateConifer(temperate) =
			ConiferMassivesCell::MassiveTemperateConifer.item()
		else {
			anyhow::bail!("expected temperate conifer item");
		};
		let temperate_samples = temperate.build_with_noise(noise);
		assert!(
			temperate_samples.geometry.inner.scale.stalk_height
				>= temperate.height.start.min(temperate.height.end)
		);
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			ConiferMassivesCell::MassiveNorthernConifer,
			ConiferMassivesCell::MassiveFriendsConifer,
			ConiferMassivesCell::MassiveLiamsConifer,
			ConiferMassivesCell::MassiveTemperateConifer,
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
			ConiferMassivesCell::MassiveNorthernConifer,
			Vec3::new(1.0, 0.0, 2.0),
			1.0,
		);
		let item = ConiferMassivesStd::with_resolved_placements(
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
	fn default_weights_yield_sparse_density_in_preview_grid() -> Result<()> {
		let span = 300.0;
		let grove = ConiferMassivesStd::default()
			.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 })
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span)));
		let cells = grove.placement_cells().len();
		let placements = grove.placements();
		let placed_share = placements.len() as f32 / cells as f32;
		assert!(
			(0.06..=0.22).contains(&placed_share),
			"expected conifer-massive fill, got {placed_share} ({}/{cells})",
			placements.len()
		);
		Ok(())
	}
}
