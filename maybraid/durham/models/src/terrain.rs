//! Durham terrain model: SDF composition, LOD generation, Avian index, render.

pub mod base_noise;
pub mod cell;
pub mod collider;
pub mod compose;
pub mod grading_graph;
pub mod index;
pub mod plugin;
pub mod presentation;
pub mod region;
pub mod region_stamps;
pub mod render;
pub mod sdf;

use crate::terrain::cell::{cell_bounds, cell_coords_for_region, HasTerrainCellLayout};
use crate::terrain::presentation::HasTerrainPresentationAssets;
use crate::terrain::render::cascade_chunk_for_cell;
use avian3d::prelude::RigidBody;
use bevy::ecs::template::template;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham_terrain::shaders::DurhamTerrainShader;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, LodScene, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use render_item::mesh::handle::Cached;

pub use base_noise::BaseTerrainNoise;
pub use cell::{MacroCellLayout, TerrainCellLayout, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
pub use collider::TerrainTrimeshCollider;
pub use compose::{create_terrain, TerrainConfig};
pub use grading_graph::GradingGraph;
pub use index::{AvianTerrainIndex, TerrainCellId, TerrainEntryStore};
pub use plugin::{register_terrain_plugin, TerrainPlugin};
pub use presentation::{
	TerrainPresentationAssets, TerrainPresenterState, TerrainRegionPresenter, TerrainStoreView,
};
pub use region_stamps::RegionStamps;
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
	pub grading: Vec<GradingGraph>,
	pub stamps: Vec<RegionStamps>,
	pub sdf: ComposedTerrain,
	pub material: Handle<DurhamTerrainShader>,
	pub res_2: u8,
}

impl Terrain {
	/// Compose an SDF from cloned base noise, region stamps, then grading.
	pub fn compose_sdf(
		base: &BaseTerrainNoise,
		stamps: &[RegionStamps],
		grading: &[GradingGraph],
	) -> ComposedTerrain {
		let mut sdf = base.sdf.clone();
		let seed = base.seed;
		for stamp_set in stamps {
			for modulation in stamp_set.modulations(seed) {
				sdf.add_elevation_modulation(Box::new(modulation));
			}
		}
		for graph in grading {
			for modulation in graph.modulations() {
				sdf.add_elevation_modulation(Box::new(modulation));
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

/// Terrain loads base noise, intersecting grading graphs, and region stamps.
impl<S> GenerationScheme<S> for Terrain
where
	S: GeneratingSpatialIndex<BaseTerrainNoise>
		+ GeneratingSpatialIndex<GradingGraph>
		+ GeneratingSpatialIndex<RegionStamps>
		+ HasTerrainCellLayout
		+ HasTerrainPresentationAssets,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		let layout = spatial_index.cell_layout().clone();
		cell_coords_for_region(region, layout.cell_size)
			.map(|(ix, iz)| {
				let bounds =
					cell_bounds(ix, iz, layout.cell_size, layout.vertical_half_extent);
				OriginalId(Id::from_cell(bounds))
			})
			.filter(|OriginalId(id)| {
				id.origin_cell_bounds().is_some_and(|b| region.intersects(&b))
			})
			.collect()
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;

		GeneratingSpatialIndex::<BaseTerrainNoise>::get_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let base = <S as SpatialIndex<BaseTerrainNoise>>::get(spatial_index, Id::Universal)?.clone();

		let grading_ids = GeneratingSpatialIndex::<GradingGraph>::get_or_generate_region(
			spatial_index,
			bounds,
			lod_ref,
		);
		let grading: Vec<GradingGraph> = grading_ids
			.iter()
			.filter_map(|(gid, _)| {
				<S as SpatialIndex<GradingGraph>>::get(spatial_index, *gid).cloned()
			})
			.collect();

		let stamp_ids = GeneratingSpatialIndex::<RegionStamps>::get_or_generate_region(
			spatial_index,
			bounds,
			lod_ref,
		);
		let stamps: Vec<RegionStamps> = stamp_ids
			.iter()
			.filter_map(|(sid, _)| {
				<S as SpatialIndex<RegionStamps>>::get(spatial_index, *sid).cloned()
			})
			.collect();

		let sdf = Self::compose_sdf(&base, &stamps, &grading);
		let assets = spatial_index.presentation_assets();
		let material = assets.material.clone();
		let res_2 = assets.res_2;

		Some((
			Self { cell: bounds, base, grading, stamps, sdf, material, res_2 },
			bounds,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
