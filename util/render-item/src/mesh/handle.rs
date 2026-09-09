use crate::mesh::cache::mesh::disk::DiskMeshCache;
use crate::{
	mesh::{
		cache::handle::map::HandleMap, cache::handle::MeshHandleCache, cache::mesh::MeshCache,
		IdentifiedMesh, MeshBuilder, MeshDispatch, MeshId,
	},
	NormalizeChunk,
};
use bevy::log::info_span;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, IoTaskPool, Task};
use chunk::cascade::CascadeChunk;
use futures::FutureExt;
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

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
impl<T: MeshBuilder + IdentifiedMesh + Clone> MeshHandle<T> {
	fn cache_mesh_blocking(&self, mesh: &Mesh, cascade_chunk: &CascadeChunk) {
		if let Some(mesh_cache) = &self.mesh_cache {
			mesh_cache.save_mesh_blocking(&self.builder, mesh, cascade_chunk);
		}
	}
}

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
///
/// `starts_per_frame` / `max_in_flight` cap async CpuShot work. Enqueue and
/// poll still run on Update: [`Self::enqueue_per_frame`] and
/// [`Self::apply_per_frame`] (plus [`Self::main_thread_budget`]) keep those
/// loops from walking or inserting the whole unmatched set in one frame.
#[derive(Resource)]
pub struct MeshFulfillBudget<T> {
	pub starts_per_frame: usize,
	pub max_in_flight: usize,
	pub max_queued: usize,
	/// When set, chunks covering this XZ are queued ahead of the rest.
	pub prefer_xz: Option<Vec3>,
	/// Unqueued dispatches to classify and look up this frame.
	pub enqueue_per_frame: usize,
	/// Handle-cache hits that may spawn a mesh child this frame.
	pub enqueue_spawns_per_frame: usize,
	/// Completed meshes to insert into [`Assets<Mesh>`] this frame.
	pub apply_per_frame: usize,
	/// Wall-time cap for enqueue lookups and poll applies (after the first).
	pub main_thread_budget: Duration,
	_marker: PhantomData<fn() -> T>,
}

const DEFAULT_ENQUEUE_PER_FRAME: usize = 32;
const DEFAULT_ENQUEUE_SPAWNS_PER_FRAME: usize = 8;
const DEFAULT_APPLY_PER_FRAME: usize = 2;
const DEFAULT_MAIN_THREAD_BUDGET: Duration = Duration::from_millis(2);

impl<T> MeshFulfillBudget<T> {
	pub fn new(starts_per_frame: usize, max_in_flight: usize, max_queued: usize) -> Self {
		Self {
			starts_per_frame,
			max_in_flight,
			max_queued,
			prefer_xz: None,
			enqueue_per_frame: DEFAULT_ENQUEUE_PER_FRAME,
			enqueue_spawns_per_frame: DEFAULT_ENQUEUE_SPAWNS_PER_FRAME,
			apply_per_frame: DEFAULT_APPLY_PER_FRAME,
			main_thread_budget: DEFAULT_MAIN_THREAD_BUDGET,
			_marker: PhantomData,
		}
	}

	pub fn with_prefer_xz(mut self, point: Vec3) -> Self {
		self.prefer_xz = Some(point);
		self
	}
}

impl<T> Default for MeshFulfillBudget<T> {
	fn default() -> Self {
		Self::new(2, 4, 64)
	}
}

#[derive(Component)]
struct MeshFulfillmentQueued;

struct MeshFulfillWork<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> {
	key: crate::mesh::cache::handle::map::ChunkMeshKey<T>,
	fetcher: MeshHandle<T>,
	chunk: CascadeChunk,
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
	completed: Vec<MeshFulfillResult<T>>,
}

impl<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static> Default
	for MeshFulfillQueue<T>
{
	fn default() -> Self {
		Self {
			queued: VecDeque::new(),
			in_flight: HashMap::new(),
			waiters: HashMap::new(),
			completed: Vec::new(),
		}
	}
}

fn spawn_mesh_child<M: Material>(
	commands: &mut Commands,
	parent_entity: Entity,
	mesh: Handle<Mesh>,
	_material: &MeshMaterial3d<M>,
) {
	commands.entity(parent_entity).insert(Mesh3d(mesh));
}

fn fulfill_mesh_now<T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static>(
	work: MeshFulfillWork<T>,
) -> MeshFulfillResult<T> {
	let _span = info_span!("mesh_fulfill").entered();
	let mesh = if let Some(mesh) = work.fetcher.fetch_cached_mesh(&work.chunk) {
		Some(mesh)
	} else {
		work.fetcher.build_mesh(&work.chunk).inspect(|mesh| {
			work.fetcher.cache_mesh_blocking(mesh, &work.chunk);
		})
	};
	MeshFulfillResult { work, mesh }
}

/// Keep viewer-column items first, then enough others to fill `consider`.
fn take_preferred_batch<T>(
	items: impl IntoIterator<Item = (bool, T)>,
	consider: usize,
	look_for_preferred: bool,
) -> Vec<(bool, T)> {
	if consider == 0 {
		return Vec::new();
	}
	let mut preferred = Vec::new();
	let mut rest = Vec::new();
	for (is_pref, item) in items {
		if is_pref {
			if preferred.len() < consider {
				preferred.push((true, item));
			}
		} else if preferred.len() + rest.len() < consider {
			rest.push((false, item));
		}
		if preferred.len() >= consider {
			break;
		}
		if !look_for_preferred && rest.len() >= consider {
			break;
		}
	}
	if preferred.len() >= consider {
		preferred
	} else {
		preferred.extend(rest);
		preferred
	}
}

fn enqueue_mesh_fulfillment<
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
	M: Material,
>(
	mut commands: Commands,
	budget: Res<MeshFulfillBudget<T>>,
	mut queue: ResMut<MeshFulfillQueue<T>>,
	query: Query<
		(Entity, &MeshDispatch<MeshHandle<T>>, &CascadeChunk, &MeshMaterial3d<M>),
		Without<MeshFulfillmentQueued>,
	>,
) {
	let prefer = budget.prefer_xz;
	let pending = take_preferred_batch(
		query.iter().map(|(entity, dispatch, chunk, material)| {
			let preferred = prefer.is_some_and(|point| chunk.column_contains_point(point));
			(preferred, (entity, dispatch, chunk, material))
		}),
		budget.enqueue_per_frame,
		prefer.is_some(),
	);

	let started = Instant::now();
	let mut looked = 0usize;
	let mut spawns = 0usize;
	for (preferred, (entity, dispatch, chunk, material)) in pending {
		if looked > 0 && started.elapsed() >= budget.main_thread_budget {
			break;
		}
		looked += 1;
		let normalized = dispatch.fetcher().normalize_chunk(chunk);
		if let Some(handle) = dispatch.fetcher().fetch_cached_mesh_handle(&normalized) {
			if spawns >= budget.enqueue_spawns_per_frame {
				break;
			}
			spawn_mesh_child(&mut commands, entity, handle, material);
			commands.entity(entity).insert(MeshFulfillmentQueued);
			spawns += 1;
			continue;
		}

		let key = crate::mesh::cache::handle::map::ChunkMeshKey::new(
			normalized.clone(),
			dispatch.fetcher().id(),
		);
		if let Some(waiters) = queue.waiters.get_mut(&key) {
			waiters.push(entity);
			commands.entity(entity).insert(MeshFulfillmentQueued);
			continue;
		}
		if queue.queued.len() >= budget.max_queued {
			break;
		}

		queue.waiters.insert(key.clone(), vec![entity]);
		let work = MeshFulfillWork { key, fetcher: dispatch.fetcher().clone(), chunk: normalized };
		if preferred {
			queue.queued.push_front(work);
		} else {
			queue.queued.push_back(work);
		}
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
		if let Some(pool) = IoTaskPool::try_get() {
			let task = pool.spawn(async move {
				let mesh = if let Some(mesh) = work.fetcher.fetch_cached_mesh(&work.chunk) {
					Some(mesh)
				} else if let Some(compute) = AsyncComputeTaskPool::try_get() {
					let fetcher = work.fetcher.clone();
					let chunk = work.chunk.clone();
					let mesh = compute.spawn(async move { fetcher.build_mesh(&chunk) }).await;
					if let Some(mesh) = mesh.as_ref() {
						work.fetcher.cache_mesh_blocking(mesh, &work.chunk);
					}
					mesh
				} else {
					work.fetcher.build_mesh(&work.chunk).inspect(|mesh| {
						work.fetcher.cache_mesh_blocking(mesh, &work.chunk);
					})
				};
				MeshFulfillResult { work, mesh }
			});
			queue.in_flight.insert(key, task);
		} else {
			queue.completed.push(fulfill_mesh_now(work));
		}
	}
}

fn poll_mesh_fulfillment<
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
	M: Material,
>(
	mut commands: Commands,
	budget: Res<MeshFulfillBudget<T>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut queue: ResMut<MeshFulfillQueue<T>>,
	materials: Query<&MeshMaterial3d<M>>,
) {
	let finished: Vec<_> = queue
		.in_flight
		.iter_mut()
		.filter_map(|(key, task)| (&mut *task).now_or_never().map(|result| (key.clone(), result)))
		.collect();
	for (key, result) in finished {
		queue.in_flight.remove(&key);
		queue.completed.push(result);
	}

	let pending = std::mem::take(&mut queue.completed);
	let started = Instant::now();
	let mut applied = 0usize;
	for result in pending {
		let Some(mesh) = result.mesh else {
			queue.waiters.remove(&result.work.key);
			continue;
		};
		if applied >= budget.apply_per_frame
			|| (applied > 0 && started.elapsed() >= budget.main_thread_budget)
		{
			queue.completed.push(MeshFulfillResult { work: result.work, mesh: Some(mesh) });
			continue;
		}
		let waiters = queue.waiters.remove(&result.work.key).unwrap_or_default();
		let handle = meshes.add(mesh);
		result.work.fetcher.cache_mesh_handle(handle.clone(), &result.work.chunk);
		for entity in waiters {
			let Ok(material) = materials.get(entity) else {
				continue;
			};
			spawn_mesh_child(&mut commands, entity, handle.clone(), material);
		}
		applied += 1;
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

		app.insert_resource(EnforcedCaches::<T> {
			handle_map: HandleMap::new(),
			disk_cache: DiskMeshCache::try_default().ok(),
		})
		.init_resource::<MeshFulfillBudget<T>>()
		.init_resource::<MeshFulfillQueue<T>>();

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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_budget_is_small() {
		let budget = MeshFulfillBudget::<()>::default();
		assert_eq!(budget.starts_per_frame, 2);
		assert_eq!(budget.max_in_flight, 4);
		assert_eq!(budget.max_queued, 64);
		assert_eq!(budget.enqueue_per_frame, 32);
		assert_eq!(budget.enqueue_spawns_per_frame, 8);
		assert_eq!(budget.apply_per_frame, 2);
		assert_eq!(budget.main_thread_budget, Duration::from_millis(2));
		assert!(budget.prefer_xz.is_none());
	}

	#[test]
	fn prefer_xz_is_optional() {
		let budget = MeshFulfillBudget::<()>::new(8, 16, 256).with_prefer_xz(Vec3::ZERO);
		assert_eq!(budget.prefer_xz, Some(Vec3::ZERO));
	}

	#[test]
	fn preferred_batch_stops_at_consider() {
		let items = (0..8).map(|i| (i % 2 == 0, i));
		let batch = take_preferred_batch(items, 3, true);
		assert_eq!(batch, vec![(true, 0), (true, 2), (true, 4)]);
	}

	#[test]
	fn preferred_batch_fills_with_rest_when_short() {
		let items = [(false, 1), (true, 2), (false, 3), (false, 4)];
		let batch = take_preferred_batch(items, 3, true);
		assert_eq!(batch, vec![(true, 2), (false, 1), (false, 3)]);
	}

	#[test]
	fn preferred_batch_zero_consider_is_empty() {
		assert!(take_preferred_batch([(true, 1)], 0, true).is_empty());
	}

	#[test]
	fn unprioritized_batch_does_not_scan_past_consider() {
		let mut seen = 0usize;
		let items = (0..64).map(|i| {
			seen += 1;
			(false, i)
		});
		let batch = take_preferred_batch(items, 4, false);
		assert_eq!(batch.len(), 4);
		assert_eq!(seen, 4);
	}
}
