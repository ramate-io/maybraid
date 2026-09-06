//! Lazy, shared references to terrain chunk meshes.
//!
//! [`TerrainChunkRef`] separates the identity of terrain geometry from any one presenter. Terrain,
//! ground-cover, and canopy entities can carry the same reference and receive the same
//! [`Handle<Mesh>`] while keeping independent materials and transforms.

use std::marker::PhantomData;

use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use lod_cascade::Chunk;
use render_item::mesh::cache::handle::map::HandleMap;
use render_item::mesh::cache::mesh::disk::DiskMeshCache;
use render_item::mesh::handle::MeshHandle;
use render_item::mesh::{IdentifiedMesh, MeshBuilder, MeshFetcher, MeshId};
use render_item::NormalizeChunk;

/// Geometry identity for a resolved terrain mesh.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerrainChunkKey {
	pub mesh_id: MeshId,
	pub chunk: CascadeChunk,
}

/// A declarative request for one terrain model sampled over one LOD chunk.
///
/// Mesh-affecting options should be carried by `T` and represented by [`IdentifiedMesh::id`].
/// [`render_item::sdf::cpu_shot::CpuShotBuilder`] already includes wall-face options in that id.
#[derive(Component, Debug, Clone)]
pub struct TerrainChunkRef<T> {
	terrain_model: T,
	chunk: Chunk,
	res_2: u8,
	key: TerrainChunkKey,
}

impl<T> TerrainChunkRef<T> {
	pub fn chunk(&self) -> &Chunk {
		&self.chunk
	}

	/// Convert the engine-neutral footprint into the existing CPU-shot request type.
	pub fn cascade_chunk(&self) -> CascadeChunk {
		Self::cascade_chunk_for(&self.chunk, self.res_2)
	}

	fn cascade_chunk_for(chunk: &Chunk, res_2: u8) -> CascadeChunk {
		let origin = chunk.bounds_min();
		let extent = chunk.extent();
		CascadeChunk {
			world: 0,
			origin,
			size: extent.max_element(),
			extent: Some(extent),
			res_2,
			omit: chunk.omit(),
		}
	}

	/// Placement for CPU-shot terrain meshes, whose vertices are local to the chunk minimum.
	pub fn transform(&self) -> Transform {
		Transform::from_translation(self.chunk.bounds_min())
	}
}

impl<T> TerrainChunkRef<T>
where
	T: IdentifiedMesh + NormalizeChunk,
{
	pub fn new(terrain_model: T, chunk: Chunk, res_2: u8) -> Self {
		let cascade_chunk = Self::cascade_chunk_for(&chunk, res_2);
		let normalized = terrain_model.normalize_chunk(&cascade_chunk);
		let key = TerrainChunkKey { mesh_id: terrain_model.id(), chunk: normalized };
		Self { terrain_model, chunk, res_2, key }
	}

	pub fn key(&self) -> &TerrainChunkKey {
		&self.key
	}
}

/// Marker and key written after a [`TerrainChunkRef`] has supplied [`Mesh3d`].
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct TerrainChunkRefResolved(pub TerrainChunkKey);

/// Marker for a successfully sampled chunk that contains no surface.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct TerrainChunkRefEmpty(pub TerrainChunkKey);

/// Shared caches used by every [`TerrainChunkRef<T>`] in an app.
#[derive(Resource, Debug, Clone)]
pub struct TerrainChunkRefCache<T>
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	handles: HandleMap<T>,
	disk: Option<DiskMeshCache<T>>,
	/// When false, a cache miss waits instead of marching. Use this when fill
	/// already owns [`Cached`] / `MeshDispatch` on the same [`HandleMap`].
	build_on_miss: bool,
}

impl<T> TerrainChunkRefCache<T>
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	pub fn new() -> Self {
		Self { handles: HandleMap::new(), disk: None, build_on_miss: true }
	}

	pub fn with_disk_cache(mut self, disk: DiskMeshCache<T>) -> Self {
		self.disk = Some(disk);
		self
	}

	pub fn with_optional_disk(mut self, disk: Option<DiskMeshCache<T>>) -> Self {
		self.disk = disk;
		self
	}

	/// Share an existing [`HandleMap`]. Fill clones handles; the first miss still builds once
	/// unless [`Self::without_build_on_miss`] is set.
	///
	/// `MeshHandle::new` allocates a private map — inject this Arc so every
	/// [`TerrainChunkRef`] and any stacked `MeshDispatch` see the same mailbox.
	pub fn with_handles(mut self, handles: HandleMap<T>) -> Self {
		self.handles = handles;
		self
	}

	/// Overlay-only: copy a handle when fill has published one; never remarch.
	pub fn without_build_on_miss(mut self) -> Self {
		self.build_on_miss = false;
		self
	}

	pub fn handles(&self) -> HandleMap<T> {
		self.handles.clone()
	}

	pub fn cached_handle(&self, terrain_ref: &TerrainChunkRef<T>) -> Option<Handle<Mesh>> {
		self.handles.get_by_id(&terrain_ref.key.chunk, terrain_ref.key.mesh_id.clone())
	}
}

impl<T> Default for TerrainChunkRefCache<T>
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}

/// Caps first-time mesh resolutions per frame. Cache hits do not consume this budget.
#[derive(Resource, Debug, Clone, Copy)]
pub struct TerrainChunkRefBudget {
	pub new_meshes_per_frame: u32,
}

impl Default for TerrainChunkRefBudget {
	fn default() -> Self {
		Self { new_meshes_per_frame: u32::MAX }
	}
}

/// Resolve terrain references independently of their material type.
type TerrainChunkRefQueryItem<'a, T> = (
	Entity,
	&'a TerrainChunkRef<T>,
	Option<&'a TerrainChunkRefResolved>,
	Option<&'a TerrainChunkRefEmpty>,
	Option<&'a Mesh3d>,
);
type TerrainChunkRefQueryFilter<T> = Or<(
	Changed<TerrainChunkRef<T>>,
	(Without<TerrainChunkRefResolved>, Without<TerrainChunkRefEmpty>),
	(With<TerrainChunkRefResolved>, Without<Mesh3d>),
)>;

pub fn fulfill_terrain_chunk_refs<T>(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	cache: Res<TerrainChunkRefCache<T>>,
	budget: Res<TerrainChunkRefBudget>,
	query: Query<TerrainChunkRefQueryItem<T>, TerrainChunkRefQueryFilter<T>>,
) where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	let mut remaining = budget.new_meshes_per_frame;

	for (entity, terrain_ref, resolved, empty, mesh) in &query {
		let key = terrain_ref.key();
		if mesh.is_some() && resolved.is_some_and(|resolved| &resolved.0 == key) {
			continue;
		}
		if empty.is_some_and(|empty| &empty.0 == key) {
			continue;
		}

		if let Some(handle) = cache.cached_handle(terrain_ref) {
			commands
				.entity(entity)
				.remove::<TerrainChunkRefEmpty>()
				.insert((Mesh3d(handle), TerrainChunkRefResolved(key.clone())));
			continue;
		}
		if !cache.build_on_miss || remaining == 0 {
			continue;
		}
		remaining -= 1;

		let fetcher = MeshHandle::new(terrain_ref.terrain_model.clone())
			.with_handle_cache(cache.handles.clone())
			.with_mesh_cache(cache.disk.clone());
		if let Some(handle) = fetcher.fetch_mesh(&mut meshes, &terrain_ref.cascade_chunk()) {
			commands
				.entity(entity)
				.remove::<TerrainChunkRefEmpty>()
				.insert((Mesh3d(handle), TerrainChunkRefResolved(key.clone())));
		} else {
			commands
				.entity(entity)
				.remove::<(Mesh3d, TerrainChunkRefResolved)>()
				.insert(TerrainChunkRefEmpty(key.clone()));
		}
	}
}

/// Installs shared handle caches and lazy fulfillment for one terrain model type.
pub struct TerrainChunkRefPlugin<T> {
	_marker: PhantomData<fn() -> T>,
}

impl<T> Default for TerrainChunkRefPlugin<T> {
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T> Plugin for TerrainChunkRefPlugin<T>
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		app.init_resource::<TerrainChunkRefCache<T>>()
			.init_resource::<TerrainChunkRefBudget>()
			.add_systems(Update, fulfill_terrain_chunk_refs::<T>);
	}
}

#[cfg(test)]
mod tests {
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::sync::Arc;

	use super::*;
	use render_item::mesh::cache::handle::map::HandleMap;

	#[derive(Clone)]
	struct CountingTerrain {
		builds: Arc<AtomicUsize>,
		ids: Arc<AtomicUsize>,
	}

	impl NormalizeChunk for CountingTerrain {}

	impl IdentifiedMesh for CountingTerrain {
		fn id(&self) -> MeshId {
			self.ids.fetch_add(1, Ordering::Relaxed);
			MeshId::new("counting-terrain".into())
		}
	}

	impl MeshBuilder for CountingTerrain {
		fn build_mesh_impl(&self, _chunk: &CascadeChunk) -> Option<Mesh> {
			self.builds.fetch_add(1, Ordering::Relaxed);
			Some(Mesh::from(Cuboid::from_length(1.0)))
		}
	}

	#[test]
	fn matching_refs_build_once_and_share_handle() -> anyhow::Result<()> {
		let builds = Arc::new(AtomicUsize::new(0));
		let ids = Arc::new(AtomicUsize::new(0));
		let model = CountingTerrain { builds: builds.clone(), ids: ids.clone() };
		let terrain_ref = TerrainChunkRef::new(model, Chunk::cube(Vec3::splat(-1.0), 2.0, None), 4);

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.add_plugins(TerrainChunkRefPlugin::<CountingTerrain>::default());

		let a = app.world_mut().spawn(terrain_ref.clone()).id();
		let b = app.world_mut().spawn(terrain_ref).id();
		app.update();

		let a_mesh = app
			.world()
			.get::<Mesh3d>(a)
			.ok_or_else(|| anyhow::anyhow!("first terrain ref was not fulfilled"))?;
		let b_mesh = app
			.world()
			.get::<Mesh3d>(b)
			.ok_or_else(|| anyhow::anyhow!("second terrain ref was not fulfilled"))?;

		assert_eq!(builds.load(Ordering::Relaxed), 1);
		assert_eq!(a_mesh.0, b_mesh.0);
		let ids_after_fulfill = ids.load(Ordering::Relaxed);
		app.update();
		assert_eq!(ids.load(Ordering::Relaxed), ids_after_fulfill);
		Ok(())
	}

	#[test]
	fn resolution_changes_mesh_identity() {
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let chunk = Chunk::cube(Vec3::splat(-1.0), 2.0, None);
		let low = TerrainChunkRef::new(model.clone(), chunk, 3);
		let high = TerrainChunkRef::new(model, chunk, 5);

		assert_ne!(low.key(), high.key());
	}

	#[derive(Clone)]
	struct EmptyTerrain {
		builds: Arc<AtomicUsize>,
	}

	impl NormalizeChunk for EmptyTerrain {}

	impl IdentifiedMesh for EmptyTerrain {
		fn id(&self) -> MeshId {
			MeshId::new("empty-terrain".into())
		}
	}

	impl MeshBuilder for EmptyTerrain {
		fn build_mesh_impl(&self, _chunk: &CascadeChunk) -> Option<Mesh> {
			self.builds.fetch_add(1, Ordering::Relaxed);
			None
		}
	}

	#[test]
	fn empty_ref_does_not_rebuild_every_frame() {
		let builds = Arc::new(AtomicUsize::new(0));
		let terrain_ref = TerrainChunkRef::new(
			EmptyTerrain { builds: builds.clone() },
			Chunk::cube(Vec3::splat(-1.0), 2.0, None),
			4,
		);
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.add_plugins(TerrainChunkRefPlugin::<EmptyTerrain>::default());
		app.world_mut().spawn(terrain_ref);

		app.update();
		app.update();

		assert_eq!(builds.load(Ordering::Relaxed), 1);
	}

	#[test]
	fn injected_handle_map_is_shared_across_cache_clones() {
		let handles = HandleMap::<CountingTerrain>::new();
		let cache = TerrainChunkRefCache::<CountingTerrain>::new().with_handles(handles.clone());
		let a = cache.handles();
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let chunk = CascadeChunk {
			world: 0,
			origin: Vec3::ZERO,
			size: 2.0,
			extent: Some(Vec3::splat(2.0)),
			res_2: 2,
			omit: None,
		};
		let mesh = Handle::default();
		a.insert(&chunk, &model, mesh.clone());
		assert!(handles.get(&chunk, &model).is_some());
	}

	#[test]
	fn cache_only_fulfill_waits_for_an_injected_handle() -> anyhow::Result<()> {
		let handles = HandleMap::<CountingTerrain>::new();
		let model = CountingTerrain {
			builds: Arc::new(AtomicUsize::new(0)),
			ids: Arc::new(AtomicUsize::new(0)),
		};
		let chunk = Chunk::cube(Vec3::splat(-1.0), 2.0, None);
		let terrain_ref = TerrainChunkRef::new(model.clone(), chunk, 2);
		let cascade = terrain_ref.cascade_chunk();

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default())).init_asset::<Mesh>();
		app.insert_resource(
			TerrainChunkRefCache::<CountingTerrain>::new()
				.with_handles(handles.clone())
				.without_build_on_miss(),
		)
		.add_plugins(TerrainChunkRefPlugin::<CountingTerrain>::default());

		let entity = app.world_mut().spawn(terrain_ref.clone()).id();
		app.update();
		assert!(app.world().get::<Mesh3d>(entity).is_none());
		assert_eq!(model.builds.load(Ordering::Relaxed), 0);

		let mesh = Handle::default();
		handles.insert(&cascade, &model, mesh.clone());
		app.update();
		let resolved = app
			.world()
			.get::<Mesh3d>(entity)
			.ok_or_else(|| anyhow::anyhow!("fill handle was not copied"))?;
		assert_eq!(resolved.0, mesh);
		assert_eq!(model.builds.load(Ordering::Relaxed), 0);
		Ok(())
	}
}
