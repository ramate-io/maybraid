use crate::mesh::cache::mesh::disk::DiskMeshCache;
use crate::{
	mesh::{
		cache::handle::map::HandleMap, cache::handle::MeshHandleCache, cache::mesh::MeshCache,
		IdentifiedMesh, MeshBuilder, MeshDispatch, MeshId,
	},
	NormalizeChunk,
};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, IoTaskPool, Task};
use chunk::cascade::CascadeChunk;
use futures::FutureExt;
use lod_first_load::{FirstLoadActivity, FirstLoadPermit};
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;

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

	fn cache_mesh_blocking(&self, mesh: &Mesh, cascade_chunk: &CascadeChunk) {
		if let Some(mesh_cache) = &self.mesh_cache {
			mesh_cache.save_mesh_blocking(&self.builder, mesh, cascade_chunk);
		}
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
			mesh_cache.save_mesh_blocking(&self.builder, mesh, cascade_chunk);
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

pub struct EnforceCachingPlugin<
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
	M: Material,
> {
	__marker: PhantomData<(T, M)>,
}

impl<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static, M: Material> Default
	for EnforceCachingPlugin<T, M>
{
	fn default() -> Self {
		Self { __marker: PhantomData }
	}
}

#[derive(Resource, Clone)]
pub struct EnforcedCaches<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> {
	handle_map: HandleMap<T>,
	disk_cache: Option<DiskMeshCache<T>>,
}

impl<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> EnforcedCaches<T> {
	/// Shared mailbox used by [`Cached`] fill. Inject this into overlay presenters
	/// ([`crate::mesh::cache::handle::map::HandleMap`] is an Arc).
	pub fn handle_map(&self) -> HandleMap<T> {
		self.handle_map.clone()
	}

	pub fn disk_cache(&self) -> Option<DiskMeshCache<T>> {
		self.disk_cache.clone()
	}
}

#[derive(Component, Clone)]
pub struct Cached<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> {
	builder: T,
}

impl<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> Cached<T> {
	pub fn new(builder: T) -> Self {
		Self { builder }
	}
}

/// Bevy system that simply rewraps the vanilla type dispathc with a mesh handle,
/// enforcing caching.
pub fn enforce_caching<
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
	M: Material,
>(
	mut commands: Commands,
	enforced_caches: Res<EnforcedCaches<T>>,
	query: Query<
		(Entity, &Cached<T>, &CascadeChunk, &Transform, &MeshMaterial3d<M>),
		Added<Cached<T>>,
	>,
) {
	for (entity, cached, _cascade_chunk, _transform, _material) in &query {
		// build a mesh handle dispatch from and insert on the entity
		let mesh_handle = MeshHandle::new(cached.builder.clone())
			.with_handle_cache(enforced_caches.handle_map.clone())
			.with_mesh_cache(enforced_caches.disk_cache.clone());
		commands
			.entity(entity)
			.insert((MeshDispatch::new(mesh_handle), Visibility::default()));
	}
}

/// Per-builder limits for cache reads and CPU mesh builds.
#[derive(Resource)]
pub struct MeshFulfillBudget<T> {
	pub starts_per_frame: usize,
	pub max_in_flight: usize,
	pub max_queued: usize,
	_marker: PhantomData<fn() -> T>,
}

impl<T> Default for MeshFulfillBudget<T> {
	fn default() -> Self {
		Self { starts_per_frame: 2, max_in_flight: 4, max_queued: 64, _marker: PhantomData }
	}
}

#[derive(Component)]
struct MeshFulfillmentQueued;

struct MeshFulfillWork<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> {
	key: crate::mesh::cache::handle::map::ChunkMeshKey<T>,
	fetcher: MeshHandle<T>,
	chunk: CascadeChunk,
	_permit: Option<FirstLoadPermit>,
}

struct MeshFulfillResult<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> {
	work: MeshFulfillWork<T>,
	mesh: Option<Mesh>,
}

#[derive(Resource)]
struct MeshFulfillQueue<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> {
	queued: VecDeque<MeshFulfillWork<T>>,
	in_flight:
		HashMap<crate::mesh::cache::handle::map::ChunkMeshKey<T>, Task<MeshFulfillResult<T>>>,
	waiters: HashMap<crate::mesh::cache::handle::map::ChunkMeshKey<T>, Vec<Entity>>,
}

impl<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> Default
	for MeshFulfillQueue<T>
{
	fn default() -> Self {
		Self { queued: VecDeque::new(), in_flight: HashMap::new(), waiters: HashMap::new() }
	}
}

fn spawn_mesh_child<M: Material>(
	commands: &mut Commands,
	parent_entity: Entity,
	mesh: Handle<Mesh>,
	material: &MeshMaterial3d<M>,
) {
	commands.entity(parent_entity).with_children(|parent| {
		parent.spawn((Mesh3d(mesh), MeshMaterial3d(material.0.clone()), Transform::default()));
	});
}

fn enqueue_mesh_fulfillment<
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
	M: Material,
>(
	mut commands: Commands,
	activity: Option<Res<FirstLoadActivity>>,
	budget: Res<MeshFulfillBudget<T>>,
	mut queue: ResMut<MeshFulfillQueue<T>>,
	query: Query<
		(Entity, &MeshDispatch<MeshHandle<T>>, &CascadeChunk, &MeshMaterial3d<M>),
		Without<MeshFulfillmentQueued>,
	>,
) {
	for (entity, dispatch, chunk, material) in &query {
		let normalized = dispatch.fetcher.normalize_chunk(chunk);
		if let Some(handle) = dispatch.fetcher.fetch_cached_mesh_handle(&normalized) {
			spawn_mesh_child(&mut commands, entity, handle, material);
			commands.entity(entity).insert(MeshFulfillmentQueued);
			continue;
		}

		let key = crate::mesh::cache::handle::map::ChunkMeshKey::new(
			normalized.clone(),
			dispatch.fetcher.id(),
		);
		if let Some(waiters) = queue.waiters.get_mut(&key) {
			waiters.push(entity);
			commands.entity(entity).insert(MeshFulfillmentQueued);
			continue;
		}
		if queue.queued.len() >= budget.max_queued {
			continue;
		}

		queue.waiters.insert(key.clone(), vec![entity]);
		queue.queued.push_back(MeshFulfillWork {
			key,
			fetcher: dispatch.fetcher.clone(),
			chunk: normalized,
			_permit: activity.as_ref().map(|activity| activity.begin()),
		});
		commands.entity(entity).insert(MeshFulfillmentQueued);
	}
}

fn start_mesh_fulfillment<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static>(
	budget: Res<MeshFulfillBudget<T>>,
	mut queue: ResMut<MeshFulfillQueue<T>>,
	entities: Query<()>,
) {
	let available = budget.max_in_flight.saturating_sub(queue.in_flight.len());
	let starts = available.min(budget.starts_per_frame);
	for _ in 0..starts {
		let Some(work) = queue.queued.pop_front() else {
			break;
		};
		let key = work.key.clone();
		let has_live_waiter = queue
			.waiters
			.get(&key)
			.is_some_and(|waiters| waiters.iter().any(|entity| entities.contains(*entity)));
		if !has_live_waiter {
			queue.waiters.remove(&key);
			continue;
		}
		let task = IoTaskPool::get().spawn(async move {
			let mesh = if let Some(mesh) = work.fetcher.fetch_cached_mesh(&work.chunk) {
				Some(mesh)
			} else {
				let fetcher = work.fetcher.clone();
				let chunk = work.chunk.clone();
				let mesh = AsyncComputeTaskPool::get()
					.spawn(async move { fetcher.build_mesh(&chunk) })
					.await;
				if let Some(mesh) = mesh.as_ref() {
					// This future is already running on the bounded I/O queue.
					// Save inline so every built mesh cannot create another
					// unbounded detached task holding a cloned mesh.
					work.fetcher.cache_mesh_blocking(mesh, &work.chunk);
				}
				mesh
			};
			MeshFulfillResult { work, mesh }
		});
		queue.in_flight.insert(key, task);
	}
}

fn poll_mesh_fulfillment<
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
	M: Material,
>(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut queue: ResMut<MeshFulfillQueue<T>>,
	materials: Query<&MeshMaterial3d<M>>,
) {
	let ready: Vec<_> = queue
		.in_flight
		.iter_mut()
		.filter_map(|(key, task)| (&mut *task).now_or_never().map(|result| (key.clone(), result)))
		.collect();

	for (key, result) in ready {
		queue.in_flight.remove(&key);
		let waiters = queue.waiters.remove(&key).unwrap_or_default();
		let Some(mesh) = result.mesh else {
			continue;
		};
		let handle = meshes.add(mesh);
		result.work.fetcher.cache_mesh_handle(handle.clone(), &result.work.chunk);
		for entity in waiters {
			let Ok(material) = materials.get(entity) else {
				continue;
			};
			spawn_mesh_child(&mut commands, entity, handle.clone(), material);
		}
	}
}

impl<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static, M: Material> Plugin
	for EnforceCachingPlugin<T, M>
{
	fn build(&self, app: &mut App) {
		log::info!(
			"Adding enforced caching plugin for {:?} {:?}",
			std::any::type_name::<T>(),
			std::any::type_name::<M>()
		);

		// insert the enforced caches resource
		app.insert_resource(EnforcedCaches::<T> {
			handle_map: HandleMap::new(),
			disk_cache: DiskMeshCache::try_default().ok(),
		})
		.init_resource::<MeshFulfillBudget<T>>()
		.init_resource::<MeshFulfillQueue<T>>();

		// Keep asset insertion and entity spawning on the main thread. Cache reads,
		// serialization, and CPU mesh construction run on AsyncComputeTaskPool.
		app.add_systems(
			Update,
			(
				enforce_caching::<T, M>,
				enqueue_mesh_fulfillment::<T, M>,
				start_mesh_fulfillment::<T>,
				poll_mesh_fulfillment::<T, M>,
			)
				.chain(),
		);
	}
}
