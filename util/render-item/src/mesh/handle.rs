use crate::mesh::cache::mesh::disk::DiskMeshCache;
use crate::{
	mesh::{
		cache::handle::map::HandleMap, cache::handle::MeshHandleCache, cache::mesh::MeshCache,
		IdentifiedMesh, MeshBuilder, MeshId,
	},
	NormalizeChunk,
};
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;

#[derive(Debug, Clone, Component)]
pub struct MeshHandle<T: MeshBuilder + IdentifiedMesh + Clone> {
	handle_cache: HandleMap<T>,
	mesh_cache: Option<DiskMeshCache<T>>,
	builder: T,
}

impl<T: MeshBuilder + IdentifiedMesh + Clone> MeshHandle<T> {
	pub fn new(builder: T) -> Self {
		Self { handle_cache: HandleMap::new(), builder, mesh_cache: None }
	}

	/// Adds a handle cache to the mesh handle.
	pub fn with_handle_cache(mut self, handle_cache: HandleMap<T>) -> Self {
		self.handle_cache = handle_cache;
		self
	}

	/// Adds a mesh cache to the mesh handle.
	pub fn with_mesh_cache(mut self, mesh_cache: Option<DiskMeshCache<T>>) -> Self {
		self.mesh_cache = mesh_cache;
		self
	}
}

/// We need to implement the identified mesh trait for this to work with the caching and fetcher.
impl<T: MeshBuilder + IdentifiedMesh + Clone> IdentifiedMesh for MeshHandle<T> {
	fn id(&self) -> MeshId {
		self.builder.id()
	}
}

/// We need to implement the normalize chunk trait to allow this to work with any of the other traits.
impl<T: MeshBuilder + IdentifiedMesh + Clone> NormalizeChunk for MeshHandle<T> {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		self.builder.normalize_chunk(cascade_chunk)
	}
}

/// We can now rederive the mesh builder trait to allow the mesh handle to be used as a mesh builder
/// which is a requirement for the mesh fetcher.
impl<T: MeshBuilder + IdentifiedMesh + Clone> MeshBuilder for MeshHandle<T> {
	fn build_mesh_impl(&self, cascade_chunk: &CascadeChunk) -> Option<Mesh> {
		self.builder.build_mesh(cascade_chunk)
	}
}

/// We implement the mesh cache trait to allow the MeshHandle<T>.
/// This is the behavior the MeshHandle<T> allows us to wrap in.
impl<T: MeshBuilder + IdentifiedMesh + Clone> MeshCache for MeshHandle<T> {
	fn cache_mesh(&self, mesh: &Mesh, cascade_chunk: &CascadeChunk) {
		if let Some(mesh_cache) = &self.mesh_cache {
			mesh_cache.save_mesh(&self.builder, mesh, cascade_chunk);
		}
	}

	fn fetch_cached_mesh(&self, cascade_chunk: &CascadeChunk) -> Option<Mesh> {
		if let Some(mesh_cache) = &self.mesh_cache {
			mesh_cache.load_mesh(&self.builder, cascade_chunk)
		} else {
			None
		}
	}
}

/// We implement the mesh handle cache trait to allow the MeshHandle<T> to cache the mesh handle.
/// This is the behavior the MeshHandle<T> allows us to wrap in around a basic builder generically.
impl<T: MeshBuilder + IdentifiedMesh + Clone> MeshHandleCache for MeshHandle<T> {
	fn cache_mesh_handle(&self, mesh_handle: Handle<Mesh>, cascade_chunk: &CascadeChunk) {
		self.handle_cache.insert(cascade_chunk, &self.builder, mesh_handle);
	}

	fn fetch_cached_mesh_handle(&self, cascade_chunk: &CascadeChunk) -> Option<Handle<Mesh>> {
		self.handle_cache.get(cascade_chunk, &self.builder)
	}
}

// We now get the blanket implementation of MeshFetcher for MeshHandle<T>.
