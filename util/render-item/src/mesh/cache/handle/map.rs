use crate::mesh::{IdentifiedMesh, MeshId};
use bevy::asset::AssetId;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use std::collections::{HashMap, HashSet};
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

	pub fn len(&self) -> usize {
		self.inner.cache.read().unwrap().len()
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Drop entries for which `keep` is false. Does not bump [`Self::generation`]:
	/// overlay waiters do not need to rescan a mailbox that only lost unused handles.
	pub fn retain(&self, mut keep: impl FnMut(&Handle<Mesh>) -> bool) -> usize {
		let mut cache = self.inner.cache.write().unwrap();
		let before = cache.len();
		cache.retain(|_, handle| keep(handle));
		before - cache.len()
	}

	/// Drop strong handles that no live [`Mesh3d`] still names.
	///
	/// Streamed-out chunks otherwise pin CpuShot CPU+GPU meshes for the rest
	/// of the process. Disk cache still serves a later rematch.
	pub fn evict_unreferenced(&self, live: &HashSet<AssetId<Mesh>>) -> usize {
		self.retain(|handle| live.contains(&handle.id()))
	}

	/// Collect [`Mesh3d`] asset ids currently attached in the world.
	pub fn live_mesh_ids(meshes: &Query<&Mesh3d>) -> HashSet<AssetId<Mesh>> {
		meshes.iter().map(|mesh| mesh.0.id()).collect()
	}
}

impl<T: IdentifiedMesh> Default for HandleMap<T> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone)]
	struct Dummy;

	impl IdentifiedMesh for Dummy {
		fn id(&self) -> MeshId {
			MeshId::new("dummy".into())
		}
	}

	fn chunk() -> CascadeChunk {
		CascadeChunk {
			world: 0,
			origin: Vec3::ZERO,
			size: 2.0,
			extent: Some(Vec3::splat(2.0)),
			res_2: 2,
			omit: None,
		}
	}

	#[test]
	fn retain_can_drop_every_entry() {
		let map = HandleMap::<Dummy>::new();
		map.insert(&chunk(), &Dummy, Handle::default());
		assert_eq!(map.len(), 1);
		assert_eq!(map.retain(|_| false), 1);
		assert!(map.is_empty());
		assert_eq!(map.generation(), 1, "evict must not look like a mailbox publish");
	}

	#[test]
	fn evict_unreferenced_keeps_live_ids() {
		let map = HandleMap::<Dummy>::new();
		let handle = Handle::default();
		let live_id = handle.id();
		map.insert(&chunk(), &Dummy, handle);
		let mut live = HashSet::new();
		live.insert(live_id);
		assert_eq!(map.evict_unreferenced(&live), 0);
		assert_eq!(map.len(), 1);
		assert_eq!(map.evict_unreferenced(&HashSet::new()), 1);
		assert!(map.is_empty());
	}
}
