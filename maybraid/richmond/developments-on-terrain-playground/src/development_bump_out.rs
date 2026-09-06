//! Canopy bump-out mesh lookup against Richmond-padded Durham terrain.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{TerrainMeshSource, WorldTerrainBuilder};
use durham_terrain_models::cascade_chunk_for_cell;
use lod_cascade::Chunk;
use richmond_development_models::TerrainWithPads;
use terrain_chunk_ref::TerrainChunkRef;

use crate::development_forest::DevelopmentExclusions;

impl<Inner: SystemParam + 'static> TerrainMeshSource for DevelopmentExclusions<'_, '_, Inner>
where
	for<'a, 'b> Inner::Item<'a, 'b>: TerrainMeshSource,
{
	fn mesh_for(
		&self,
		bounds: Aabb3d,
		cell_size: f32,
	) -> Option<TerrainChunkRef<WorldTerrainBuilder>> {
		if let Some(terrain) = self.development.store.padded_terrain_for(bounds) {
			let size = terrain.cell.max.x - terrain.cell.min.x;
			if (size - cell_size).abs() <= cell_size.max(1e-3) * 0.25 {
				return Some(padded_terrain_ref(terrain));
			}
		}

		self.inner.mesh_for(bounds, cell_size)
	}
}

pub(crate) fn padded_terrain_ref(
	terrain: &TerrainWithPads,
) -> TerrainChunkRef<WorldTerrainBuilder> {
	let cascade = cascade_chunk_for_cell(terrain.cell, terrain.res_2);
	let extent = cascade.extent.unwrap_or(Vec3::splat(cascade.size));
	let chunk = Chunk::from_min_max(cascade.origin, cascade.origin + extent, None);
	TerrainChunkRef::new(terrain.mesh_builder(), chunk, terrain.res_2)
}

#[cfg(test)]
mod tests {
	use super::*;
	use durham_terrain_models::{ComposedTerrain, TerrainSdf};
	use render_item::mesh::IdentifiedMesh;
	use std::sync::Arc;

	#[test]
	fn padded_bump_out_ref_matches_presented_terrain_mesh_key() {
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(160.0, 100.0, 160.0));
		let terrain = TerrainWithPads {
			cell: bounds,
			sdf: Arc::new(ComposedTerrain::from_terrain(TerrainSdf::new(42, 80.0))),
			material: Handle::default(),
			res_2: 5,
			wall_faces: render_item::sdf::cpu_shot::WallFaces::ALL,
			pad_count: 0,
		};
		let terrain_ref = padded_terrain_ref(&terrain);

		assert_eq!(terrain_ref.key().mesh_id, terrain.mesh_builder().id());
		assert_eq!(terrain_ref.key().chunk, terrain_ref.cascade_chunk());
	}
}
