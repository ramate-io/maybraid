//! [`HcsgStorage`]: completed, immutable values shared across threads.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};

use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::Resource;

use crate::gen::{Id, Version};

use super::node_store::{NodeStore, StoredEntry, DEFAULT_BASE_SCALE};

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
	next_version: AtomicU64,
}

struct TypedStore<T> {
	nodes: RwLock<NodeStore<Arc<T>>>,
}

trait ErasedStore: Send + Sync {
	fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
	fn reset(&self, revision: u64);
}

impl<T: HcsgValue> ErasedStore for TypedStore<T> {
	fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
		self
	}

	fn reset(&self, revision: u64) {
		write(&self.nodes).reset(revision);
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

impl HcsgStorage {
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
	}
}
