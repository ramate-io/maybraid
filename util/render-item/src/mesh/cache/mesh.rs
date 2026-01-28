pub mod disk;

use crate::mesh::IdentifiedMesh;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;

pub trait MeshCache: Clone + IdentifiedMesh {
	/// Caches a mesh.
	fn cache_mesh(&self, mesh: &Mesh, cascade_chunk: &CascadeChunk);

	/// Fetches a mesh from the cache.
	fn fetch_cached_mesh(&self, cascade_chunk: &CascadeChunk) -> Option<Mesh>;
}
