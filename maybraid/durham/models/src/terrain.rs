//! Durham terrain model: SDF composition, LOD generation, Avian index, render.

pub mod base_noise;
pub mod cell;
pub mod collider;
pub mod config;
pub mod index;
pub mod jersey;
pub mod jersey_modulation;
pub mod plugin;
pub mod presentation;
pub mod render;
pub mod sdf;

use crate::terrain::cell::original_ids_for_origin_cells;
use crate::terrain::jersey::{
	original_ids_for_canyon_leaves, original_ids_for_massif_leaves,
	original_ids_for_plateau_leaves, original_ids_for_pocket_water_leaves,
	original_ids_for_rolling_leaves, original_ids_for_valley_leaves, CanyonControllerCell,
	MassifControllerCell, PlateauControllerCell, PocketWaterControllerCell, RollingControllerCell,
	ValleyControllerCell,
};
use crate::terrain::render::cascade_chunk_for_cell;
use avian3d::prelude::RigidBody;
use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham_terrain::shaders::DurhamTerrainShader;
use jersey_terrain_stamps::JerseyModulation;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, LodScene, OriginalId};
use lod::lod_ref::LodRef;
use render_item::mesh::handle::Cached;

pub use base_noise::BaseTerrainNoise;
pub use cell::{MacroCellLayout, TerrainCellLayout, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
pub use collider::TerrainTrimeshCollider;
pub use config::TerrainConfig;
pub use index::{AvianTerrainIndex, TerrainCellId, TerrainEntryStore};
pub use jersey::{
	CanyonControllerLayout, CanyonStampCell, JerseyStampConfigs, MassifControllerLayout,
	MassifStampCell, PlateauControllerLayout, PlateauStampCell, PocketWaterControllerLayout,
	PocketWaterStampCell, RollingControllerLayout, RollingStampCell, ValleyControllerLayout,
	ValleyStampCell,
};
pub use plugin::{register_terrain_plugin, TerrainPlugin};
pub use presentation::{
	TerrainPresentationAssets, TerrainPresenterState, TerrainRegionPresenter, TerrainStoreView,
};
pub use render::TerrainRenderItem;
pub use sdf::{ComposedTerrain, ElevationModulation, TerrainSdf};

/// Top-level terrain cell model.
#[derive(Debug, Clone, Component)]
pub struct Terrain {
	pub cell: Aabb3d,
	pub base: BaseTerrainNoise,
	/// Flattened jersey leaf modulations (deterministic family + leaf Id order).
	pub modulations: Vec<JerseyModulation>,
	/// Leaf AABBs whose stamps contributed (debug / HUD), all families.
	pub jersey_leaves: Vec<Aabb3d>,
	pub sdf: ComposedTerrain,
	pub material: Handle<DurhamTerrainShader>,
	pub res_2: u8,
}

impl Terrain {
	pub fn compose_sdf(
		base: &BaseTerrainNoise,
		modulations: &[JerseyModulation],
	) -> ComposedTerrain {
		let mut sdf = base.sdf.clone();
		for modulation in modulations {
			sdf.add_elevation_modulation(Box::new(modulation.clone()));
		}
		ComposedTerrain::from_terrain(sdf)
	}

	pub fn scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = Transform::from_translation(chunk.origin);
		let sdf = self.sdf.clone();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(sdf.clone())))
			MeshMaterial3d::<DurhamTerrainShader>({material.clone()})
			template(move |_ctx| Ok(RigidBody::Static))
			TerrainTrimeshCollider
		}
	}
}

impl LodScene for Terrain {
	fn scene_with_lod(&self, _lod_ref: &LodRef) -> impl Scene + 'static {
		self.scene()
	}
}

macro_rules! pull_family_stamps {
	($spatial_index:expr, $lod_ref:expr, $bounds:expr, $leaves_fn:path, $Stamp:ty, $mods:expr, $leaf_out:expr) => {{
		let mut leaf_ids = $leaves_fn($spatial_index, $bounds);
		leaf_ids.sort_by(|a, b| a.0.cmp(&b.0));
		for OriginalId(lid) in leaf_ids {
			let stamp = GeneratingSpatialIndex::<$Stamp>::get_one_or_generate(
				$spatial_index,
				lid,
				$lod_ref,
			)?;
			$leaf_out.push(stamp.cell);
			$mods.extend(stamp.modulations.iter().cloned());
		}
	}};
}

/// Terrain loads base noise and per-family jersey leaf stamps directly.
impl<S> GenerationScheme<S> for Terrain
where
	S: GeneratingSpatialIndex<BaseTerrainNoise>
		+ GeneratingSpatialIndex<JerseyStampConfigs>
		+ GeneratingSpatialIndex<PlateauStampCell>
		+ GeneratingSpatialIndex<PlateauControllerCell>
		+ GeneratingSpatialIndex<PlateauControllerLayout>
		+ GeneratingSpatialIndex<MassifStampCell>
		+ GeneratingSpatialIndex<MassifControllerCell>
		+ GeneratingSpatialIndex<MassifControllerLayout>
		+ GeneratingSpatialIndex<CanyonStampCell>
		+ GeneratingSpatialIndex<CanyonControllerCell>
		+ GeneratingSpatialIndex<CanyonControllerLayout>
		+ GeneratingSpatialIndex<PocketWaterStampCell>
		+ GeneratingSpatialIndex<PocketWaterControllerCell>
		+ GeneratingSpatialIndex<PocketWaterControllerLayout>
		+ GeneratingSpatialIndex<RollingStampCell>
		+ GeneratingSpatialIndex<RollingControllerCell>
		+ GeneratingSpatialIndex<RollingControllerLayout>
		+ GeneratingSpatialIndex<ValleyStampCell>
		+ GeneratingSpatialIndex<ValleyControllerCell>
		+ GeneratingSpatialIndex<ValleyControllerLayout>
		+ GeneratingSpatialIndex<TerrainCellLayout>
		+ GeneratingSpatialIndex<TerrainPresentationAssets>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;

		let base = GeneratingSpatialIndex::<BaseTerrainNoise>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?
		.clone();

		let mut modulations = Vec::new();
		let mut jersey_leaves = Vec::new();

		// Fixed family order so neighboring Terrain cells compose identically.
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_plateau_leaves,
			PlateauStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_massif_leaves,
			MassifStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_canyon_leaves,
			CanyonStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_pocket_water_leaves,
			PocketWaterStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_rolling_leaves,
			RollingStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_valley_leaves,
			ValleyStampCell,
			modulations,
			jersey_leaves
		);

		let sdf = Self::compose_sdf(&base, &modulations);
		let assets = GeneratingSpatialIndex::<TerrainPresentationAssets>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let material = assets.material.clone();
		let res_2 = assets.res_2;

		Some((
			Self { cell: bounds, base, modulations, jersey_leaves, sdf, material, res_2 },
			bounds,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
