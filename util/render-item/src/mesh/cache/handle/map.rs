use crate::mesh::{IdentifiedMesh, MeshId};
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
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

struct HandleMapInner<T: IdentifiedMesh> {
	cache: RwLock<HashMap<ChunkMeshKey<T>, Handle<Mesh>>>,
	generation: AtomicU64,
}

/// Read-only mailbox ownership snapshot; not a complete set of asset owners.
#[derive(Debug)]
pub struct HandleMapSnapshot {
	pub entries: usize,
	/// HashMap entry capacity, not allocated bytes.
	pub entry_capacity: usize,
	pub strong_handle_entries: usize,
	pub unique_mesh_ids: HashSet<bevy::asset::AssetId<Mesh>>,
}

/// Shared [`Handle<Mesh>`] mailbox. [`Self::generation`] ticks on each insert so
/// overlay waiters can skip lookups while the map is quiet.
#[derive(Clone)]
pub struct HandleMap<T: IdentifiedMesh> {
	inner: Arc<HandleMapInner<T>>,
}

impl<T: IdentifiedMesh> std::fmt::Debug for HandleMap<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("HandleMap").field("generation", &self.generation()).finish()
	}
}

impl<T: IdentifiedMesh> HandleMap<T> {
	/// Explicit O(entries) census. Does not clone strong handles, promote entries,
	/// tick generation, or wait for a busy writer. None means busy or poisoned.
	pub fn try_snapshot(&self) -> Option<HandleMapSnapshot> {
		let cache = self.inner.cache.try_read().ok()?;
		Some(HandleMapSnapshot {
			entries: cache.len(),
			entry_capacity: cache.capacity(),
			strong_handle_entries: cache
				.values()
				.filter(|h| matches!(h, Handle::Strong(_)))
				.count(),
			unique_mesh_ids: cache.values().map(Handle::id).collect(),
		})
	}

	/// Aliases share one mailbox and must not be added as separate owners.
	pub fn shares_storage_with(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.inner, &other.inner)
	}

	pub fn new() -> Self {
		Self {
			inner: Arc::new(HandleMapInner {
				cache: RwLock::new(HashMap::new()),
				generation: AtomicU64::new(0),
			}),
		}
	}

	/// Monotonic mailbox version. Overlay fulfill uses this to skip waiter walks.
	pub fn generation(&self) -> u64 {
		self.inner.generation.load(Ordering::Acquire)
	}

	pub fn get(&self, chunk: &CascadeChunk, mesh_builder: &T) -> Option<Handle<Mesh>> {
		self.get_by_id(chunk, mesh_builder.id())
	}

	/// Look up a mesh whose identity has already been computed by the caller.
	pub fn get_by_id(&self, chunk: &CascadeChunk, mesh_id: MeshId) -> Option<Handle<Mesh>> {
		let cache = self.inner.cache.read().unwrap();
		cache.get(&ChunkMeshKey::new(chunk.clone(), mesh_id)).cloned()
	}

	pub fn insert(&self, chunk: &CascadeChunk, mesh_builder: &T, mesh: Handle<Mesh>) {
		let mut cache = self.inner.cache.write().unwrap();
		let key = ChunkMeshKey::new(chunk.clone(), mesh_builder.id());
		cache.insert(key, mesh);
		self.inner.generation.fetch_add(1, Ordering::Release);
	}
}

impl<T: IdentifiedMesh> Default for HandleMap<T> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use crate::mesh::cache::handle::map::HandleMap;
	use crate::mesh::{IdentifiedMesh, MeshId};
	use bevy::prelude::*;
	use chunk::cascade::CascadeChunk;

	#[derive(Clone)]
	struct Builder;

	impl IdentifiedMesh for Builder {
		fn id(&self) -> MeshId {
			MeshId::new("snapshot-test".into())
		}
	}

	#[test]
	fn snapshot_deduplicates_ids_and_preserves_mailbox() -> anyhow::Result<()> {
		let map = HandleMap::<Builder>::new();
		let chunk = CascadeChunk {
			world: 0,
			origin: Vec3::ZERO,
			size: 1.0,
			extent: None,
			res_2: 1,
			omit: None,
		};
		let mut next = chunk.clone();
		next.origin = Vec3::X;
		let handle = Handle::<Mesh>::default();
		map.insert(&chunk, &Builder, handle.clone());
		map.insert(&next, &Builder, handle);
		let generation = map.generation();
		let snapshot = map.try_snapshot().ok_or_else(|| anyhow::anyhow!("unavailable snapshot"))?;
		assert_eq!(snapshot.entries, 2);
		assert_eq!(snapshot.unique_mesh_ids.len(), 1);
		assert!(snapshot.entry_capacity >= 2);
		assert_eq!(map.generation(), generation);
		assert!(map.shares_storage_with(&map.clone()));
		assert!(!map.shares_storage_with(&HandleMap::new()));
		assert!(map.get_by_id(&chunk, Builder.id()).is_some());
		Ok(())
	}

	#[test]
	fn busy_writer_is_reported_without_waiting() -> anyhow::Result<()> {
		let map = HandleMap::<Builder>::new();
		let _writer = map.inner.cache.write().map_err(|_| anyhow::anyhow!("poisoned test map"))?;
		assert!(map.try_snapshot().is_none());
		Ok(())
	}
}
