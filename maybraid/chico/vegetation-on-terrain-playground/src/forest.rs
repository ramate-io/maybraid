//! Durham-height forest stream. Grows on [`TerrainGroveSample`] so tiles sit on real ground.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_forests::TerrainHeightSource;
use chico_groves::TerrainGroveSample;
use durham_terrain_models::{
	TerrainCellLayout, TerrainEntryStore, TerrainLodCell, TerrainLodIndex, TerrainStoreView,
};
use lod::gen::GeneratingSpatialIndex;
use lod::lod_ref::LodRef;

use crate::bump_out::{
	terrain_chunk_ref, terrain_for_cell_size, TerrainMeshSource, WorldTerrainBuilder,
};
use crate::groves::OwnedDurhamTerrain;
use crate::WorldBaseTerrain;
use terrain_chunk_ref::TerrainChunkRef;

/// Seek overlapping Durham origin cells, then snapshot composed height for grow.
#[derive(SystemParam)]
pub struct DurhamHeight<'w> {
	lod: ResMut<'w, TerrainLodIndex>,
	store: Res<'w, TerrainEntryStore>,
	layout: Res<'w, TerrainCellLayout>,
	base: Res<'w, WorldBaseTerrain>,
}

impl TerrainHeightSource for DurhamHeight<'_> {
	fn ensure_and_sample(
		&mut self,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) -> Option<impl chico_groves::GroveWorldSample + Clone + Send + Sync + 'static> {
		let _ = GeneratingSpatialIndex::<TerrainLodCell>::get_or_generate_region(
			&mut *self.lod,
			bounds,
			lod_ref,
		);
		Some(TerrainGroveSample::new(OwnedDurhamTerrain::new(
			self.store.height_snapshot(),
			self.layout.clone(),
			self.base.0.clone(),
		)))
	}
}

impl TerrainMeshSource for DurhamHeight<'_> {
	fn mesh_for(
		&self,
		bounds: Aabb3d,
		cell_size: f32,
	) -> Option<TerrainChunkRef<WorldTerrainBuilder>> {
		let view = TerrainStoreView::new(&self.store, &self.layout);
		terrain_for_cell_size(&view, bounds, cell_size).map(terrain_chunk_ref)
	}
}
