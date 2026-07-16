//! Durham terrain model: SDF composition, LOD generation, Avian index, render.

pub mod base_noise;
pub mod cell;
pub mod cell_noise;
pub mod collider;
pub mod config;
pub mod index;
pub mod jersey_compose;
pub mod jersey_configs;
pub mod jersey_layers;
pub mod jersey_modulation;
pub mod plugin;
pub mod presentation;
pub mod render;
pub mod sdf;

use crate::terrain::cell::original_ids_for_origin_cells;
use crate::terrain::render::cascade_chunk_for_cell;
use avian3d::prelude::RigidBody;
use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham_terrain::shaders::DurhamTerrainShader;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, LodScene, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use render_item::mesh::handle::Cached;

pub use base_noise::BaseTerrainNoise;
pub use cell::{
	JerseyStampCellLayout, MacroCellLayout, TerrainCellLayout, JERSEY_STAMP_CELL_SIZE,
	JERSEY_STAMP_GRID_OFFSET, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE,
};
pub use cell_noise::CellTerrainNoise;
pub use collider::TerrainTrimeshCollider;
pub use config::TerrainConfig;
pub use index::{AvianTerrainIndex, TerrainCellId, TerrainEntryStore};
pub use jersey_compose::{JerseyFamilySummary, JerseyModulations};
pub use jersey_configs::JerseyLayerConfigs;
pub use jersey_layers::{
	CanyonLayer, PlateauCapLayer, PocketWaterLayer, RollingGroundLayer, RuggedMassifLayer,
	ValleyBasinLayer,
};
pub use plugin::{register_terrain_plugin, TerrainPlugin};
pub use presentation::{
	TerrainPresentationAssets, TerrainPresenterState, TerrainRegionPresenter, TerrainStoreView,
};
pub use render::TerrainRenderItem;
pub use sdf::{ComposedTerrain, ElevationModulation, TerrainSdf};

/// Top-level terrain cell model.
///
/// Built by pulling intersecting generation deps from the spatial index, cloning
/// them in, and composing a per-cell SDF for sampling / presentation.
#[derive(Debug, Clone, Component)]
pub struct Terrain {
	pub cell: Aabb3d,
	pub base: BaseTerrainNoise,
	pub jersey: Vec<JerseyModulations>,
	pub sdf: ComposedTerrain,
	pub material: Handle<DurhamTerrainShader>,
	pub res_2: u8,
}

impl Terrain {
	/// Compose an SDF from cloned base noise and intersecting jersey modulations.
	pub fn compose_sdf(base: &BaseTerrainNoise, jersey: &[JerseyModulations]) -> ComposedTerrain {
		let mut sdf = base.sdf.clone();
		for cell in jersey {
			for modulation in &cell.modulations {
				sdf.add_elevation_modulation(Box::new(modulation.clone()));
			}
		}
		ComposedTerrain::from_terrain(sdf)
	}

	/// Visual scene for one cell: cascade chunk + cached SDF mesh dispatch.
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

/// Terrain loads base noise and intersecting jersey modulation cells.
impl<S> GenerationScheme<S> for Terrain
where
	S: GeneratingSpatialIndex<BaseTerrainNoise>
		+ GeneratingSpatialIndex<JerseyModulations>
		+ GeneratingSpatialIndex<TerrainCellLayout>
		+ GeneratingSpatialIndex<TerrainPresentationAssets>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;

		GeneratingSpatialIndex::<BaseTerrainNoise>::get_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let base =
			<S as SpatialIndex<BaseTerrainNoise>>::get(spatial_index, Id::Universal)?.clone();

		let mut jersey_ids = GeneratingSpatialIndex::<JerseyModulations>::get_or_generate_region(
			spatial_index,
			bounds,
			lod_ref,
		);
		// Keep Id order when composing so neighboring Terrain cells apply
		// non-commutative jersey ops identically.
		jersey_ids.sort_by(|(a, _), (b, _)| a.cmp(b));
		let jersey: Vec<JerseyModulations> = jersey_ids
			.iter()
			.filter_map(|(jid, _)| {
				<S as SpatialIndex<JerseyModulations>>::get(spatial_index, *jid).cloned()
			})
			.collect();

		let sdf = Self::compose_sdf(&base, &jersey);
		GeneratingSpatialIndex::<TerrainPresentationAssets>::get_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let assets =
			<S as SpatialIndex<TerrainPresentationAssets>>::get(spatial_index, Id::Universal)?;
		let material = assets.material.clone();
		let res_2 = assets.res_2;

		Some((Self { cell: bounds, base, jersey, sdf, material, res_2 }, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
