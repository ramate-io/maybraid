use crate::mesh::{IdentifiedMesh, MeshId};
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ChunkMeshKey<T: IdentifiedMesh> {
	chunk: CascadeChunk,
	mesh_id: MeshId,
	phantom: std::marker::PhantomData<T>,
}

impl<T: IdentifiedMesh> Display for ChunkMeshKey<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"ChunkMeshKey::<{}>::(chunk: {:?}, mesh_id: {:?})",
			std::any::type_name::<T>(),
			self.chunk,
			self.mesh_id
		)
	}
}

impl<T: IdentifiedMesh> PartialEq for ChunkMeshKey<T> {
	fn eq(&self, other: &Self) -> bool {
		self.chunk == other.chunk && self.mesh_id == other.mesh_id
	}
}

impl<T: IdentifiedMesh> Eq for ChunkMeshKey<T> {}

impl<T: IdentifiedMesh> Hash for ChunkMeshKey<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.chunk.hash(state);
		self.mesh_id.hash(state);
		// PhantomData doesn't need to be hashed
	}
}

impl<T: IdentifiedMesh> ChunkMeshKey<T> {
	pub fn new(chunk: CascadeChunk, mesh_id: MeshId) -> Self {
		Self { chunk, mesh_id, phantom: std::marker::PhantomData }
	}
}

#[derive(Debug, Clone)]
pub struct HandleMap<T: IdentifiedMesh> {
	cache: Arc<RwLock<HashMap<ChunkMeshKey<T>, Handle<Mesh>>>>,
}

impl<T: IdentifiedMesh> HandleMap<T> {
	pub fn new() -> Self {
		Self { cache: Arc::new(RwLock::new(HashMap::new())) }
	}

	pub fn get(&self, chunk: &CascadeChunk, mesh_builder: &T) -> Option<Handle<Mesh>> {
		self.get_by_id(chunk, mesh_builder.id())
	}

	/// Look up a mesh whose identity has already been computed by the caller.
	pub fn get_by_id(&self, chunk: &CascadeChunk, mesh_id: MeshId) -> Option<Handle<Mesh>> {
		let cache = self.cache.read().unwrap();
		cache.get(&ChunkMeshKey::new(chunk.clone(), mesh_id)).cloned()
	}

	pub fn insert(&self, chunk: &CascadeChunk, mesh_builder: &T, mesh: Handle<Mesh>) {
		let mut cache = self.cache.write().unwrap();
		let key = ChunkMeshKey::new(chunk.clone(), mesh_builder.id());
		cache.insert(key, mesh);
	}
}

impl<T: IdentifiedMesh> Default for HandleMap<T> {
	fn default() -> Self {
		Self::new()
	}
}
