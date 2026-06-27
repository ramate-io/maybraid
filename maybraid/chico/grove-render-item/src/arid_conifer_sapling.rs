//! [`RenderItem`] for populated Arid Conifer Sapling groves ([#327](https://github.com/ramate-io/maybraid/issues/327)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_trees::friends_conifer::FriendsConifer;
use chico_sbs_trees::liams_conifer::LiamsConifer;
use chico_sbs_trees::northern_conifer::NorthernConifer;
use chico_vegetation_shaders::ChicoStickMaterial;
use clap::Args;
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

#[cfg(test)]
use crate::conifer_sapling::ConiferSaplingStd;
use crate::skipped_mesh_material::{
	SkippedLeafMeshMaterial, SkippedStickMeshMaterial as GroveSkippedStickMeshMaterial,
};
use chico_groves::arid_conifer_sapling::{
	definition, AridConiferSaplingCell, AridConiferSaplingItem,
};
use chico_groves::{
	patch_spawned_leaf_material, placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GroveFrontend, GroveWorldSample, WithPalette, DEFAULT_GROVE_EXTENT_XZ,
};

/// Typical [`ChicoStickMaterial`] / [`StandardMaterial`] Arid Conifer Sapling instance.
pub type AridConiferSaplingStd = AridConiferSapling<
	ChicoStickMaterial,
	GroveSkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
	FlatTerrainSample,
>;

/// Arid Conifer Sapling grove preview (sparse dry Friend's, Northern, and rare Liam's conifers).
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct AridConiferSapling<StickM, StickS, LeafM, LeafS, Terrain = FlatTerrainSample>
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
		default_value = "0,0.1,1.0,1",
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
	resolved_placements: Option<Vec<GroveCellVariant<AridConiferSaplingCell>>>,

	#[arg(skip)]
	__marker: PhantomData<fn() -> (StickM, LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS, Terrain> Default
	for AridConiferSapling<StickM, StickS, LeafM, LeafS, Terrain>
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
	AridConiferSapling<StickM, StickS, LeafM, LeafS, Terrain>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
	Terrain: GroveWorldSample + Clone + Send + Sync + 'static + Default + clap::Args,
{
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<AridConiferSaplingCell>>,
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

	pub fn placements(&self) -> Vec<GroveCellVariant<AridConiferSaplingCell>> {
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
	for AridConiferSapling<StickM, StickS, LeafM, LeafS, Terrain>
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
				AridConiferSaplingItem::FriendsConifer(conifer) => {
					let geometry = conifer.build_with_noise(build_noise);
					let mut tree = FriendsConifer::<StickM, StickS, LeafM, LeafS>::default();
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
				AridConiferSaplingItem::NorthernConifer(conifer) => {
					let geometry = conifer.build_with_noise(build_noise);
					let mut tree = NorthernConifer::<StickM, StickS, LeafM, LeafS>::default();
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
				AridConiferSaplingItem::LiamsConifer(conifer) => {
					let geometry = conifer.build_with_noise(build_noise);
					let mut tree = LiamsConifer::<StickM, StickS, LeafM, LeafS>::default();
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
	use chico_sbs_geometry::{FriendsConiferSbs, LiamsConiferSbs};

	#[test]
	fn tree_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));

		let AridConiferSaplingItem::FriendsConifer(friend) =
			AridConiferSaplingCell::DryFriendSapling.item()
		else {
			anyhow::bail!("expected dry friend sapling item");
		};
		let friend_geometry = friend.build_with_noise(noise);
		assert!(friend_geometry.scale.stalk_height >= friend.height.start.min(friend.height.end));
		assert!(friend_geometry.scale.stalk_height <= friend.height.start.max(friend.height.end));
		assert!(
			friend_geometry.rings.spacing
				>= friend.canopy_density.start.min(friend.canopy_density.end)
		);
		assert!(
			friend_geometry.rings.spacing
				<= friend.canopy_density.start.max(friend.canopy_density.end)
		);
		assert_eq!(friend_geometry, {
			let mut expected = FriendsConiferSbs::default();
			expected.rings.spacing = friend_geometry.rings.spacing;
			expected.scale.stalk_height = friend_geometry.scale.stalk_height;
			expected.scale.stalk_base_radius = friend_geometry.scale.stalk_base_radius;
			expected.canopy_noise = noise;
			expected
		});

		let AridConiferSaplingItem::NorthernConifer(northern) =
			AridConiferSaplingCell::DryNorthernSapling.item()
		else {
			anyhow::bail!("expected dry northern sapling item");
		};
		let northern_geometry = northern.build_with_noise(noise);
		assert!(
			northern_geometry.liams.scale.stalk_height
				>= northern.height.start.min(northern.height.end)
		);
		assert!(
			northern_geometry.liams.scale.stalk_height
				<= northern.height.start.max(northern.height.end)
		);

		let AridConiferSaplingItem::LiamsConifer(liams) =
			AridConiferSaplingCell::DryLiamsConiferSapling.item()
		else {
			anyhow::bail!("expected dry liams sapling item");
		};
		let liams_geometry = liams.build_with_noise(noise);
		assert!(liams_geometry.scale.stalk_height >= liams.height.start.min(liams.height.end));
		assert!(liams_geometry.scale.stalk_height <= liams.height.start.max(liams.height.end));
		assert!(
			liams_geometry.rings.spacing
				>= liams.canopy_density.start.min(liams.canopy_density.end)
		);
		assert!(
			liams_geometry.rings.spacing
				<= liams.canopy_density.start.max(liams.canopy_density.end)
		);
		assert_eq!(liams_geometry, {
			let mut expected = LiamsConiferSbs::default();
			expected.rings.spacing = liams_geometry.rings.spacing;
			expected.scale.stalk_height = liams_geometry.scale.stalk_height;
			expected.scale.stalk_base_radius = liams_geometry.scale.stalk_base_radius;
			expected.canopy_noise = noise;
			expected
		});
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [
			AridConiferSaplingCell::DryFriendSapling,
			AridConiferSaplingCell::DryNorthernSapling,
			AridConiferSaplingCell::WispyDryFriendSapling,
			AridConiferSaplingCell::WispyDryNorthernSapling,
			AridConiferSaplingCell::BareDryFriendSapling,
			AridConiferSaplingCell::BareDryNorthernSapling,
			AridConiferSaplingCell::DryLiamsConiferSapling,
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
		let arid = AridConiferSaplingStd::default().with_extent(extent.clone());
		let placements = arid.placements();
		assert!(!placements.is_empty(), "expected visible arid sapling preview");
		Ok(())
	}

	#[test]
	fn default_weights_yield_sparser_placements_than_conifer_sapling() -> Result<()> {
		let span = DEFAULT_GROVE_EXTENT_XZ;
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
		let arid = AridConiferSaplingStd::default()
			.with_terrain(terrain)
			.with_extent(extent.clone());
		let humid = ConiferSaplingStd::default().with_extent(extent);
		let cells = arid.placement_cells().len();
		let arid_placements = arid.placements();
		let humid_placements = humid.placements();
		let arid_share = arid_placements.len() as f32 / cells as f32;
		let humid_share = humid_placements.len() as f32 / cells as f32;
		assert!(
			(0.06..=0.26).contains(&arid_share),
			"expected sparse arid fill, got {arid_share} ({}/{cells})",
			arid_placements.len()
		);
		assert!(
			arid_placements.len() < humid_placements.len(),
			"expected arid ({}) sparser than humid conifer sapling ({}) on the same extent",
			arid_placements.len(),
			humid_placements.len()
		);
		assert!(
			arid_share < humid_share,
			"expected arid share ({arid_share}) below humid share ({humid_share})"
		);
		assert!(!arid_placements.is_empty());
		Ok(())
	}
}
