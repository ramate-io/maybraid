//! [`HcsgStorage`]: completed, immutable values shared across threads.

use std::any::{Any, TypeId};
use std::collections::HashMap;
#[cfg(debug_assertions)]
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};
#[cfg(debug_assertions)]
use std::sync::{Mutex, MutexGuard};

use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::Resource;

use crate::gen::{Id, Version};

use super::context::GenerationScheme;
use super::node_store::{expand_region, NodeStore, StoredEntry, DEFAULT_BASE_SCALE};

/// Any value HCSG can store.
pub trait HcsgValue: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> HcsgValue for T {}

/// A lock was held elsewhere. Retry next frame; this is never "empty".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Busy;

/// Cloneable handle over one [`NodeStore`] of `Arc<T>` per type.
///
/// The registry lock is held only long enough to clone a store's `Arc`; a
/// store's lock only for lookup and publication. Neither is held while a
/// value is generated. Values never change once published: replacing one
/// publishes a new [`Version`] from a counter shared by every store.
#[derive(Resource, Clone, Default)]
pub struct HcsgStorage(Arc<Registry>);

#[derive(Default)]
struct Registry {
	stores: RwLock<HashMap<TypeId, Arc<dyn ErasedStore>>>,
	base_scales: RwLock<HashMap<TypeId, DVec3>>,
	retention_margins: RwLock<HashMap<TypeId, DVec3>>,
	next_version: AtomicU64,
	/// Keys removed by a sweep; debug rebuilds check this set.
	#[cfg(debug_assertions)]
	evicted: Mutex<HashSet<(TypeId, Id)>>,
	top_rebuilds: AtomicU64,
	nested_rebuilds: AtomicU64,
}

struct TypedStore<T> {
	nodes: RwLock<NodeStore<Arc<T>>>,
}

trait ErasedStore: Send + Sync {
	fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
	fn reset(&self, revision: u64);
	fn ids_outside(&self, regions: &[Aabb3d]) -> Vec<Id>;
	fn remove_ids(&self, ids: &[Id], revision: u64);
	fn len(&self) -> usize;
	fn type_name(&self) -> &'static str;
}

impl<T: HcsgValue> ErasedStore for TypedStore<T> {
	fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
		self
	}

	fn reset(&self, revision: u64) {
		write(&self.nodes).reset(revision);
	}

	fn ids_outside(&self, regions: &[Aabb3d]) -> Vec<Id> {
		read(&self.nodes).ids_outside(regions)
	}

	fn remove_ids(&self, ids: &[Id], revision: u64) {
		write(&self.nodes).remove_ids(ids, revision);
	}

	fn len(&self) -> usize {
		read(&self.nodes).len()
	}

	fn type_name(&self) -> &'static str {
		std::any::type_name::<T>()
	}
}

/// Values are immutable and published whole, so a poisoned lock still guards
/// a consistent store.
fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
	lock.read().unwrap_or_else(PoisonError::into_inner)
}

fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
	lock.write().unwrap_or_else(PoisonError::into_inner)
}

fn try_read<T>(lock: &RwLock<T>) -> Result<RwLockReadGuard<'_, T>, Busy> {
	match lock.try_read() {
		Ok(guard) => Ok(guard),
		Err(TryLockError::Poisoned(poisoned)) => Ok(poisoned.into_inner()),
		Err(TryLockError::WouldBlock) => Err(Busy),
	}
}

#[cfg(debug_assertions)]
fn lock_mutex<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
	lock.lock().unwrap_or_else(PoisonError::into_inner)
}

impl HcsgStorage {
	/// Records `T`'s index scale and retention margin before its first publish.
	pub(crate) fn register_scheme<T: GenerationScheme>(&self) {
		let type_id = TypeId::of::<T>();
		{
			let mut scales = write(&self.0.base_scales);
			if !scales.contains_key(&type_id) {
				scales.insert(type_id, T::INDEX_SCALE);
			}
		}
		{
			let mut margins = write(&self.0.retention_margins);
			if !margins.contains_key(&type_id) {
				margins.insert(type_id, T::RETENTION_MARGIN);
			}
		}
	}

	/// Sets `T`'s spatial base scale, rebuilding its index if it exists.
	pub fn configure<T: HcsgValue>(&self, base_scale: DVec3) -> &Self {
		write(&self.0.base_scales).insert(TypeId::of::<T>(), base_scale);
		if let Some(store) = self.store::<T>() {
			write(&store.nodes).rebuild_index(base_scale);
		}
		self
	}

	fn downcast<T: HcsgValue>(store: Arc<dyn ErasedStore>) -> Option<Arc<TypedStore<T>>> {
		store.into_any().downcast().ok()
	}

	fn store<T: HcsgValue>(&self) -> Option<Arc<TypedStore<T>>> {
		let store = read(&self.0.stores).get(&TypeId::of::<T>()).cloned()?;
		Self::downcast(store)
	}

	fn try_store<T: HcsgValue>(&self) -> Result<Option<Arc<TypedStore<T>>>, Busy> {
		let store = try_read(&self.0.stores)?.get(&TypeId::of::<T>()).cloned();
		Ok(store.and_then(Self::downcast))
	}

	fn store_or_create<T: HcsgValue>(&self) -> Option<Arc<TypedStore<T>>> {
		if let Some(store) = self.store::<T>() {
			return Some(store);
		}
		let base_scale = read(&self.0.base_scales)
			.get(&TypeId::of::<T>())
			.copied()
			.unwrap_or(DEFAULT_BASE_SCALE);
		let store = write(&self.0.stores)
			.entry(TypeId::of::<T>())
			.or_insert_with(|| {
				Arc::new(TypedStore::<T> { nodes: RwLock::new(NodeStore::new(base_scale)) })
			})
			.clone();
		Self::downcast(store)
	}

	fn stamp(&self) -> u64 {
		self.0.next_version.fetch_add(1, Ordering::Relaxed) + 1
	}

	pub fn get<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>> {
		let store = self.store::<T>()?;
		let value = read(&store.nodes).value(id).cloned();
		value
	}

	pub fn entry<T: HcsgValue>(&self, id: Id) -> Option<StoredEntry<Arc<T>>> {
		let store = self.store::<T>()?;
		let entry = read(&store.nodes).entry(id).cloned();
		entry
	}

	pub fn contains<T: HcsgValue>(&self, id: Id) -> bool {
		self.store::<T>().is_some_and(|store| read(&store.nodes).contains(id))
	}

	/// Stored ids of `T` whose bounds intersect `region`, in id order.
	pub fn overlapping<T: HcsgValue>(&self, region: Aabb3d) -> Vec<Id> {
		self.store::<T>()
			.map(|store| read(&store.nodes).overlapping(region))
			.unwrap_or_default()
	}

	/// Bumped by every publish, removal and clear of `T`.
	pub fn membership_revision<T: HcsgValue>(&self) -> u64 {
		self.store::<T>().map_or(0, |store| read(&store.nodes).membership_revision())
	}

	/// [`Self::entry`] for the frame: never waits on a lock.
	pub fn try_entry<T: HcsgValue>(&self, id: Id) -> Result<Option<StoredEntry<Arc<T>>>, Busy> {
		let Some(store) = self.try_store::<T>()? else {
			return Ok(None);
		};
		let entry = try_read(&store.nodes)?.entry(id).cloned();
		Ok(entry)
	}

	/// [`Self::overlapping`] for the frame: never waits on a lock.
	pub fn try_overlapping<T: HcsgValue>(&self, region: Aabb3d) -> Result<Vec<Id>, Busy> {
		let Some(store) = self.try_store::<T>()? else {
			return Ok(Vec::new());
		};
		let ids = try_read(&store.nodes)?.overlapping(region);
		Ok(ids)
	}

	/// [`Self::membership_revision`] for the frame: never waits on a lock.
	pub fn try_membership_revision<T: HcsgValue>(&self) -> Result<u64, Busy> {
		let Some(store) = self.try_store::<T>()? else {
			return Ok(0);
		};
		let revision = try_read(&store.nodes)?.membership_revision();
		Ok(revision)
	}

	/// Publishes `value` and its spatial entry together under a fresh version.
	pub fn publish<T: HcsgValue>(&self, id: Id, value: Arc<T>, bounds: Aabb3d) -> Version {
		let stamp = self.stamp();
		let version = Version(stamp);
		if let Some(store) = self.store_or_create::<T>() {
			write(&store.nodes).put(id, StoredEntry { value, bounds, version }, stamp);
		}
		version
	}

	/// [`Self::publish`], unless `stale` holds once `T`'s store is locked.
	///
	/// A session reset cancels its jobs before clearing, so a value built for
	/// the old session either lands before the clear or is dropped here.
	pub fn publish_unless<T: HcsgValue>(
		&self,
		id: Id,
		value: Arc<T>,
		bounds: Aabb3d,
		stale: impl Fn() -> bool,
	) -> Option<Version> {
		let stamp = self.stamp();
		let store = self.store_or_create::<T>()?;
		let mut nodes = write(&store.nodes);
		if stale() {
			return None;
		}
		nodes.put(id, StoredEntry { value, bounds, version: Version(stamp) }, stamp);
		Some(Version(stamp))
	}

	/// Stores a session root at `Id::Universal`.
	pub fn seed<T: HcsgValue>(&self, value: T, bounds: Aabb3d) -> Version {
		self.publish(Id::Universal, Arc::new(value), bounds)
	}

	pub fn remove<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>> {
		let stamp = self.stamp();
		let store = self.store::<T>()?;
		let removed = write(&store.nodes).remove(id, stamp).map(|entry| entry.value);
		removed
	}

	/// Drops every value of `T`.
	pub fn clear<T: HcsgValue>(&self) {
		let stamp = self.stamp();
		let store = read(&self.0.stores).get(&TypeId::of::<T>()).cloned();
		if let Some(store) = store {
			store.reset(stamp);
		}
		self.forget_evictions_of(TypeId::of::<T>());
	}

	/// How many values of `T` are stored, including `Id::Universal`.
	pub fn len<T: HcsgValue>(&self) -> usize {
		self.store::<T>().map_or(0, |store| read(&store.nodes).len())
	}

	/// `(type name, len)` for every store, sorted by name. Diagnostics.
	pub fn store_sizes(&self) -> Vec<(&'static str, usize)> {
		let mut sizes: Vec<_> = read(&self.0.stores)
			.values()
			.map(|store| (store.type_name(), store.len()))
			.collect();
		sizes.sort_by_key(|(name, _)| *name);
		sizes
	}

	/// Top-level and nested rebuilds of values this store previously evicted.
	pub fn rebuilds_after_eviction(&self) -> (u64, u64) {
		(
			self.0.top_rebuilds.load(Ordering::Relaxed),
			self.0.nested_rebuilds.load(Ordering::Relaxed),
		)
	}

	/// Removes every `T` whose bounds overlap none of `regions`. `Id::Universal`
	/// is kept. Removal bumps the membership revision as [`Self::remove`] does.
	pub fn retain_overlapping<T: HcsgValue>(&self, regions: &[Aabb3d]) {
		let Some(store) = self.store::<T>() else {
			return;
		};
		let removed = store.ids_outside(regions);
		if removed.is_empty() {
			return;
		}
		let stamp = self.stamp();
		store.remove_ids(&removed, stamp);
		self.record_evictions(removed.into_iter().map(|id| (TypeId::of::<T>(), id)).collect());
	}

	/// Worker sweep: each stored type is kept in the union of `regions_by_type`
	/// for that type, expanded by the type's retention margin. A type no live
	/// subscription reaches is cleared, except `Id::Universal` entries.
	pub(super) fn retain_reached(&self, regions_by_type: &HashMap<TypeId, Vec<Aabb3d>>) {
		if regions_by_type.is_empty() {
			self.clear_evictions();
		}
		let stores: Vec<(TypeId, Arc<dyn ErasedStore>)> = read(&self.0.stores)
			.iter()
			.map(|(type_id, store)| (*type_id, store.clone()))
			.collect();
		for (type_id, store) in stores {
			let margin = self.retention_margin_of(type_id);
			let expanded: Vec<Aabb3d> = regions_by_type
				.get(&type_id)
				.into_iter()
				.flatten()
				.map(|region| expand_region(*region, margin))
				.collect();
			let removed = store.ids_outside(&expanded);
			if removed.is_empty() {
				continue;
			}
			let stamp = self.stamp();
			store.remove_ids(&removed, stamp);
			self.record_evictions(removed.into_iter().map(|id| (type_id, id)).collect());
		}
	}

	fn base_scale_of(&self, type_id: TypeId) -> DVec3 {
		read(&self.0.base_scales).get(&type_id).copied().unwrap_or(DEFAULT_BASE_SCALE)
	}

	fn retention_margin_of(&self, type_id: TypeId) -> DVec3 {
		read(&self.0.retention_margins)
			.get(&type_id)
			.copied()
			.unwrap_or_else(|| self.base_scale_of(type_id))
	}

	#[cfg(test)]
	pub(crate) fn index_scale_of<T: HcsgValue>(&self) -> DVec3 {
		self.base_scale_of(TypeId::of::<T>())
	}

	fn record_evictions(&self, removed: Vec<(TypeId, Id)>) {
		if removed.is_empty() {
			return;
		}
		#[cfg(debug_assertions)]
		lock_mutex(&self.0.evicted).extend(removed);
		#[cfg(not(debug_assertions))]
		let _ = removed;
	}

	/// True if this key was swept; removes it so the set cannot grow forever.
	pub(super) fn take_evicted(&self, type_id: TypeId, id: Id) -> bool {
		#[cfg(debug_assertions)]
		{
			lock_mutex(&self.0.evicted).remove(&(type_id, id))
		}
		#[cfg(not(debug_assertions))]
		{
			let _ = (type_id, id);
			false
		}
	}

	pub(super) fn record_rebuild(&self, nested: bool) {
		if nested {
			self.0.nested_rebuilds.fetch_add(1, Ordering::Relaxed);
		} else {
			self.0.top_rebuilds.fetch_add(1, Ordering::Relaxed);
		}
	}

	fn forget_evictions_of(&self, type_id: TypeId) {
		#[cfg(debug_assertions)]
		lock_mutex(&self.0.evicted).retain(|(stored, _)| *stored != type_id);
		#[cfg(not(debug_assertions))]
		let _ = type_id;
	}

	/// Drops every recorded eviction. Called when no live subscription remains
	/// (epoch or last unsubscribe) so a new session does not count as rebuilds.
	pub fn clear_evictions(&self) {
		#[cfg(debug_assertions)]
		lock_mutex(&self.0.evicted).clear();
	}
}
