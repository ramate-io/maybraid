//! Durham-height forest stream. Grows on [`TerrainGroveSample`] so tiles sit on real ground.
//!
//! Present is get-only: skip grow when the overlapping Durham cell is not in
//! [`TerrainEntryStore`]. Do not `get_or_generate` [`TerrainLodCell`]s from here
//! ([#720](https://github.com/ramate-io/maybraid/issues/720) / [#719](https://github.com/ramate-io/maybraid/issues/719) §2).

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_forests::TerrainHeightSource;
use chico_groves::TerrainGroveSample;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore, TerrainStoreView};
use lod::lod_ref::LodRef;

use crate::bump_out::{
	terrain_chunk_ref, terrain_for_cell_size, TerrainMeshSource, WorldTerrainBuilder,
};
use crate::groves::OwnedDurhamTerrain;
use crate::WorldBaseTerrain;
use terrain_chunk_ref::TerrainChunkRef;

/// Snapshot composed height when the overlapping Durham cell is already stored.
#[derive(SystemParam)]
pub struct DurhamHeight<'w> {
	store: Res<'w, TerrainEntryStore>,
	layout: Res<'w, TerrainCellLayout>,
	base: Res<'w, WorldBaseTerrain>,
}

/// Whether [`TerrainEntryStore`] already has composed height at the center of `bounds`.
pub fn durham_store_ready_for(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	bounds: Aabb3d,
) -> bool {
	let center = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
	store.composed_height_at(layout, center.x, center.z).is_some()
}

impl TerrainHeightSource for DurhamHeight<'_> {
	fn ensure_and_sample(
		&mut self,
		bounds: Aabb3d,
		_lod_ref: &LodRef,
	) -> Option<impl chico_groves::GroveWorldSample + Clone + Send + Sync + 'static> {
		if !durham_store_ready_for(&self.store, &self.layout, bounds) {
			return None;
		}
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn get_only_skips_when_store_has_no_cell() {
		let store = TerrainEntryStore::default();
		let layout = TerrainCellLayout::default();
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		assert!(!durham_store_ready_for(&store, &layout, bounds));
	}
}
