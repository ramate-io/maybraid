//! Durham terrain model: SDF composition, LOD generation, Avian index, render.

pub mod base_noise;
pub mod cell;
pub mod collider;
pub mod config;
pub mod index;
pub mod jersey;
pub mod jersey_modulation;
pub mod marazion;
pub mod plugin;
pub mod presentation;
pub mod render;
pub mod sdf;

use crate::terrain::cell::original_ids_for_origin_cells;
use crate::terrain::jersey::{
	original_ids_for_canyon_high_pass_leaves, original_ids_for_canyon_low_pass_leaves,
	original_ids_for_massif_high_pass_leaves, original_ids_for_massif_low_pass_leaves,
	original_ids_for_plateau_high_pass_leaves, original_ids_for_plateau_low_pass_leaves,
	original_ids_for_pocket_water_high_pass_leaves, original_ids_for_pocket_water_low_pass_leaves,
	original_ids_for_rolling_high_pass_leaves, original_ids_for_rolling_low_pass_leaves,
	original_ids_for_valley_high_pass_leaves, original_ids_for_valley_low_pass_leaves,
};
use crate::terrain::marazion::{original_ids_for_marazion_lake_leaves, MarazionLakeCell};
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
	CanyonHighPassControllerCell, CanyonHighPassControllerLayout, CanyonHighPassStampCell,
	CanyonLowPassControllerCell, CanyonLowPassControllerLayout, CanyonLowPassStampCell,
	JerseyControllerLayouts, JerseyStampConfigs, MassifHighPassControllerCell,
	MassifHighPassControllerLayout, MassifHighPassStampCell, MassifLowPassControllerCell,
	MassifLowPassControllerLayout, MassifLowPassStampCell, PlateauControllerLayout,
	PlateauHighPassControllerCell, PlateauHighPassControllerLayout, PlateauHighPassStampCell,
	PlateauLowPassControllerCell, PlateauLowPassControllerLayout, PlateauLowPassStampCell,
	PocketWaterHighPassControllerCell, PocketWaterHighPassControllerLayout,
	PocketWaterHighPassStampCell, PocketWaterLowPassControllerCell,
	PocketWaterLowPassControllerLayout, PocketWaterLowPassStampCell, RollingHighPassControllerCell,
	RollingHighPassControllerLayout, RollingHighPassStampCell, RollingLowPassControllerCell,
	RollingLowPassControllerLayout, RollingLowPassStampCell, ValleyHighPassControllerCell,
	ValleyHighPassControllerLayout, ValleyHighPassStampCell, ValleyLowPassControllerCell,
	ValleyLowPassControllerLayout, ValleyLowPassStampCell,
};
pub use jersey::{
	CanyonLowPassStampCell as CanyonStampCell, MassifLowPassStampCell as MassifStampCell,
	PlateauLowPassStampCell as PlateauStampCell, PocketWaterLowPassStampCell as PocketWaterStampCell,
	RollingLowPassStampCell as RollingStampCell, ValleyLowPassStampCell as ValleyStampCell,
};
pub use marazion::{
	MarazionLakeCell as MarazionLakeStampCell, MarazionWatershedConfigs, PocketCell,
	PrePocketCell, PrePocketLayout,
};
pub use plugin::{register_terrain_plugin, TerrainPlugin};
pub use presentation::{
	TerrainPresentationAssets, TerrainPresenterState, TerrainRegionPresenter, TerrainStoreView,
};
pub use render::TerrainRenderItem;
pub use sdf::{ComposedTerrain, ElevationModulation, TerrainSdf};

/// Jersey-composed terrain **before** Marazion watershed stamps.
#[derive(Debug, Clone, Component)]
pub struct PreWatershedTerrain {
	pub cell: Aabb3d,
	pub base: BaseTerrainNoise,
	pub modulations: Vec<JerseyModulation>,
	pub jersey_leaves: Vec<Aabb3d>,
	pub sdf: ComposedTerrain,
}

impl PreWatershedTerrain {
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
}

/// Final terrain cell: pre-watershed heightfield + Marazion lake stamps.
#[derive(Debug, Clone, Component)]
pub struct Terrain {
	pub cell: Aabb3d,
	pub base: BaseTerrainNoise,
	/// Flattened jersey + marazion leaf modulations.
	pub modulations: Vec<JerseyModulation>,
	/// Leaf AABBs whose jersey stamps contributed (debug / HUD).
	pub jersey_leaves: Vec<Aabb3d>,
	/// Leaf AABBs whose Marazion lake stamps contributed.
	pub marazion_leaves: Vec<Aabb3d>,
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
			if stamp.modulations.is_empty() {
				continue;
			}
			$leaf_out.push(stamp.cell);
			$mods.extend(stamp.modulations.iter().cloned());
		}
	}};
}

/// Pre-watershed: base noise + jersey landform stamps (including pocket-water height).
impl<S> GenerationScheme<S> for PreWatershedTerrain
where
	S: GeneratingSpatialIndex<BaseTerrainNoise>
		+ GeneratingSpatialIndex<JerseyStampConfigs>
		+ GeneratingSpatialIndex<PlateauHighPassStampCell>
		+ GeneratingSpatialIndex<PlateauHighPassControllerCell>
		+ GeneratingSpatialIndex<PlateauHighPassControllerLayout>
		+ GeneratingSpatialIndex<PlateauLowPassStampCell>
		+ GeneratingSpatialIndex<PlateauLowPassControllerCell>
		+ GeneratingSpatialIndex<PlateauLowPassControllerLayout>
		+ GeneratingSpatialIndex<MassifHighPassStampCell>
		+ GeneratingSpatialIndex<MassifHighPassControllerCell>
		+ GeneratingSpatialIndex<MassifHighPassControllerLayout>
		+ GeneratingSpatialIndex<MassifLowPassStampCell>
		+ GeneratingSpatialIndex<MassifLowPassControllerCell>
		+ GeneratingSpatialIndex<MassifLowPassControllerLayout>
		+ GeneratingSpatialIndex<CanyonHighPassStampCell>
		+ GeneratingSpatialIndex<CanyonHighPassControllerCell>
		+ GeneratingSpatialIndex<CanyonHighPassControllerLayout>
		+ GeneratingSpatialIndex<CanyonLowPassStampCell>
		+ GeneratingSpatialIndex<CanyonLowPassControllerCell>
		+ GeneratingSpatialIndex<CanyonLowPassControllerLayout>
		+ GeneratingSpatialIndex<PocketWaterHighPassStampCell>
		+ GeneratingSpatialIndex<PocketWaterHighPassControllerCell>
		+ GeneratingSpatialIndex<PocketWaterHighPassControllerLayout>
		+ GeneratingSpatialIndex<PocketWaterLowPassStampCell>
		+ GeneratingSpatialIndex<PocketWaterLowPassControllerCell>
		+ GeneratingSpatialIndex<PocketWaterLowPassControllerLayout>
		+ GeneratingSpatialIndex<RollingHighPassStampCell>
		+ GeneratingSpatialIndex<RollingHighPassControllerCell>
		+ GeneratingSpatialIndex<RollingHighPassControllerLayout>
		+ GeneratingSpatialIndex<RollingLowPassStampCell>
		+ GeneratingSpatialIndex<RollingLowPassControllerCell>
		+ GeneratingSpatialIndex<RollingLowPassControllerLayout>
		+ GeneratingSpatialIndex<ValleyHighPassStampCell>
		+ GeneratingSpatialIndex<ValleyHighPassControllerCell>
		+ GeneratingSpatialIndex<ValleyHighPassControllerLayout>
		+ GeneratingSpatialIndex<ValleyLowPassStampCell>
		+ GeneratingSpatialIndex<ValleyLowPassControllerCell>
		+ GeneratingSpatialIndex<ValleyLowPassControllerLayout>
		+ GeneratingSpatialIndex<TerrainCellLayout>,
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

		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_plateau_high_pass_leaves,
			PlateauHighPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_massif_high_pass_leaves,
			MassifHighPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_canyon_high_pass_leaves,
			CanyonHighPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_pocket_water_high_pass_leaves,
			PocketWaterHighPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_rolling_high_pass_leaves,
			RollingHighPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_valley_high_pass_leaves,
			ValleyHighPassStampCell,
			modulations,
			jersey_leaves
		);

		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_plateau_low_pass_leaves,
			PlateauLowPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_massif_low_pass_leaves,
			MassifLowPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_canyon_low_pass_leaves,
			CanyonLowPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_pocket_water_low_pass_leaves,
			PocketWaterLowPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_rolling_low_pass_leaves,
			RollingLowPassStampCell,
			modulations,
			jersey_leaves
		);
		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_valley_low_pass_leaves,
			ValleyLowPassStampCell,
			modulations,
			jersey_leaves
		);

		let sdf = Self::compose_sdf(&base, &modulations);
		Some((
			Self {
				cell: bounds,
				base,
				modulations,
				jersey_leaves,
				sdf,
			},
			bounds,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

/// Final terrain: pre-watershed + Marazion lake leaf stamps.
impl<S> GenerationScheme<S> for Terrain
where
	S: GeneratingSpatialIndex<PreWatershedTerrain>
		+ GeneratingSpatialIndex<MarazionWatershedConfigs>
		+ GeneratingSpatialIndex<PrePocketLayout>
		+ GeneratingSpatialIndex<PrePocketCell>
		+ GeneratingSpatialIndex<PocketCell>
		+ GeneratingSpatialIndex<MarazionLakeCell>
		+ GeneratingSpatialIndex<TerrainCellLayout>
		+ GeneratingSpatialIndex<TerrainPresentationAssets>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;

		let pre = GeneratingSpatialIndex::<PreWatershedTerrain>::get_one_or_generate(
			spatial_index,
			id,
			lod_ref,
		)?
		.clone();

		let mut modulations = pre.modulations.clone();
		let jersey_leaves = pre.jersey_leaves.clone();
		let mut marazion_leaves = Vec::new();

		pull_family_stamps!(
			spatial_index,
			lod_ref,
			bounds,
			original_ids_for_marazion_lake_leaves,
			MarazionLakeCell,
			modulations,
			marazion_leaves
		);

		let sdf = Self::compose_sdf(&pre.base, &modulations);
		let assets = GeneratingSpatialIndex::<TerrainPresentationAssets>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let material = assets.material.clone();
		let res_2 = assets.res_2;

		Some((
			Self {
				cell: bounds,
				base: pre.base,
				modulations,
				jersey_leaves,
				marazion_leaves,
				sdf,
				material,
				res_2,
			},
			bounds,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
