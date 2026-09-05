//! Canopy bump-out presentation against Richmond-padded Durham terrain.

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_forests::{CanopyBumpOut, ForestIndex, MediumCanopyBumpOut, MEDIUM_BUMP_OUT_CELL_XZ};
use chico_vegetation_on_terrain_playground::{
	bump_out_from_cell, bump_out_noise, fine_terrain_for, medium_terrain_for, terrain_chunk_ref,
	CanopyBumpOutPresenterState, MediumCanopyBumpOutPresenterState, WorldTerrainBuilder,
};
use durham_terrain_models::{cascade_chunk_for_cell, TerrainStoreView};
use lod::gen::{Id, SpatialIndex, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod_cascade::Chunk;
use richmond_development_models::{DevelopmentIndex, TerrainWithPads};
use terrain_chunk_ref::TerrainChunkRef;

/// Presents far-canopy overlays on the exact padded mesh when that replacement
/// is active, falling back to the matching raw Durham cell elsewhere.
#[derive(SystemParam)]
pub struct DevelopmentCanopyBumpOutPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, CanopyBumpOutPresenterState>,
	development: DevelopmentIndex<'w>,
	forest: Res<'w, ForestIndex>,
}

impl DevelopmentCanopyBumpOutPresenter<'_, '_> {
	fn terrain_ref_for(&self, bounds: Aabb3d) -> Option<TerrainChunkRef<WorldTerrainBuilder>> {
		if let Some(terrain) = self.development.store.padded_terrain_for(bounds) {
			return Some(Self::padded_terrain_ref(terrain));
		}

		let view =
			TerrainStoreView::new(self.development.terrain_store(), self.development.layout());
		fine_terrain_for(&view, bounds).map(terrain_chunk_ref)
	}

	fn padded_terrain_ref(terrain: &TerrainWithPads) -> TerrainChunkRef<WorldTerrainBuilder> {
		let cascade = cascade_chunk_for_cell(terrain.cell, terrain.res_2);
		let extent = cascade.extent.unwrap_or(Vec3::splat(cascade.size));
		let chunk = Chunk::from_min_max(cascade.origin, cascade.origin + extent, None);
		TerrainChunkRef::new(terrain.mesh_builder(), chunk, terrain.res_2)
	}
}

impl RegionPresenter<CanopyBumpOut, ForestIndex> for DevelopmentCanopyBumpOutPresenter<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		let cell = SpatialIndex::<CanopyBumpOut>::get(&*self.forest, id)?;
		let terrain_ref = self.terrain_ref_for(cell.bounds)?;
		self.state.presented_version_for_terrain(id, terrain_ref.key())
	}

	fn handle(&mut self, id: Id, version: Version, cell: &CanopyBumpOut, _lod_ref: &LodRef) {
		let Some(bump_out) = bump_out_from_cell(cell, bump_out_noise(&self.forest.noise)) else {
			return;
		};
		let Some(terrain_ref) = self.terrain_ref_for(cell.bounds) else {
			return;
		};
		self.state.present(&mut self.commands, id, version, bump_out, terrain_ref);
	}

	fn hide(&mut self, id: Id) {
		self.state.hide(&mut self.commands, id);
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.is_hidden(id)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented_ids()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		self.state.remove_stale(&mut self.commands, wanted);
	}
}

/// Presents canopy overlays on Richmond-padded medium terrain when available,
/// falling back to the matching raw 320 m Durham cell.
#[derive(SystemParam)]
pub struct DevelopmentMediumCanopyBumpOutPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, MediumCanopyBumpOutPresenterState>,
	development: DevelopmentIndex<'w>,
	forest: Res<'w, ForestIndex>,
}

impl DevelopmentMediumCanopyBumpOutPresenter<'_, '_> {
	fn terrain_ref_for(&self, bounds: Aabb3d) -> Option<TerrainChunkRef<WorldTerrainBuilder>> {
		if let Some(terrain) = self.development.store.padded_terrain_for(bounds) {
			let size = terrain.cell.max.x - terrain.cell.min.x;
			if (size - MEDIUM_BUMP_OUT_CELL_XZ).abs() < 1e-2 {
				return Some(DevelopmentCanopyBumpOutPresenter::padded_terrain_ref(terrain));
			}
		}

		let view =
			TerrainStoreView::new(self.development.terrain_store(), self.development.layout());
		medium_terrain_for(&view, bounds).map(terrain_chunk_ref)
	}
}

impl RegionPresenter<MediumCanopyBumpOut, ForestIndex>
	for DevelopmentMediumCanopyBumpOutPresenter<'_, '_>
{
	fn presented_version(&self, id: Id) -> Option<Version> {
		let cell = SpatialIndex::<MediumCanopyBumpOut>::get(&*self.forest, id)?;
		let terrain_ref = self.terrain_ref_for(cell.0.bounds)?;
		self.state.presented_version_for_terrain(id, terrain_ref.key())
	}

	fn handle(&mut self, id: Id, version: Version, cell: &MediumCanopyBumpOut, _lod_ref: &LodRef) {
		let Some(bump_out) = bump_out_from_cell(&cell.0, bump_out_noise(&self.forest.noise)) else {
			return;
		};
		let Some(terrain_ref) = self.terrain_ref_for(cell.0.bounds) else {
			return;
		};
		self.state.present(&mut self.commands, id, version, bump_out, terrain_ref);
	}

	fn hide(&mut self, id: Id) {
		self.state.hide(&mut self.commands, id);
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.is_hidden(id)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented_ids()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		self.state.remove_stale(&mut self.commands, wanted);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use durham_terrain_models::{ComposedTerrain, TerrainSdf};
	use render_item::mesh::IdentifiedMesh;
	use render_item::sdf::cpu_shot::WallFaces;
	use std::sync::Arc;

	#[test]
	fn padded_bump_out_ref_matches_presented_terrain_mesh_key() {
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(160.0, 100.0, 160.0));
		let terrain = TerrainWithPads {
			cell: bounds,
			sdf: Arc::new(ComposedTerrain::from_terrain(TerrainSdf::new(42, 80.0))),
			material: Handle::default(),
			res_2: 5,
			wall_faces: WallFaces::ALL,
			pad_count: 0,
		};
		let terrain_ref = DevelopmentCanopyBumpOutPresenter::padded_terrain_ref(&terrain);

		assert_eq!(terrain_ref.key().mesh_id, terrain.mesh_builder().id());
		assert_eq!(terrain_ref.key().chunk, terrain_ref.cascade_chunk());
	}
}
