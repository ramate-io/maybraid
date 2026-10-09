//! Typed store registry and per-type [`NodeStore`] handles.

use std::any::{Any, TypeId};
use std::collections::HashMap;
#[cfg(debug_assertions)]
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};
#[cfg(debug_assertions)]
use std::sync::{Mutex, MutexGuard};

use bevy::math::DVec3;

use crate::gen::Id;

use super::super::node_store::{NodeStore, DEFAULT_BASE_SCALE};
use super::{Busy, HcsgStorage, HcsgValue};

#[derive(Default)]
pub(super) struct Registry {
	pub stores: RwLock<HashMap<TypeId, Arc<dyn ErasedStore>>>,
	pub base_scales: RwLock<HashMap<TypeId, DVec3>>,
	pub next_version: AtomicU64,
	#[cfg(debug_assertions)]
	pub evicted: Mutex<HashSet<(TypeId, Id)>>,
	pub top_rebuilds: AtomicU64,
	pub nested_rebuilds: AtomicU64,
}

pub(super) struct TypedStore<T> {
	pub nodes: RwLock<NodeStore<Arc<T>>>,
}

pub(super) trait ErasedStore: Send + Sync {
	fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
	fn reset(&self, revision: u64);
	fn ids_outside(&self, regions: &[bevy::math::bounding::Aabb3d]) -> Vec<Id>;
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

	fn ids_outside(&self, regions: &[bevy::math::bounding::Aabb3d]) -> Vec<Id> {
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

pub(super) fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
	lock.read().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
	lock.write().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn try_read<T>(lock: &RwLock<T>) -> Result<RwLockReadGuard<'_, T>, Busy> {
	match lock.try_read() {
		Ok(guard) => Ok(guard),
		Err(TryLockError::Poisoned(poisoned)) => Ok(poisoned.into_inner()),
		Err(TryLockError::WouldBlock) => Err(Busy),
	}
}

#[cfg(debug_assertions)]
pub(super) fn lock_mutex<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
	lock.lock().unwrap_or_else(PoisonError::into_inner)
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

	pub(super) fn downcast<T: HcsgValue>(store: Arc<dyn ErasedStore>) -> Option<Arc<TypedStore<T>>> {
		store.into_any().downcast().ok()
	}

	pub(super) fn store<T: HcsgValue>(&self) -> Option<Arc<TypedStore<T>>> {
		let store = read(&self.0.stores).get(&TypeId::of::<T>()).cloned()?;
		Self::downcast(store)
	}

	pub(super) fn try_store<T: HcsgValue>(&self) -> Result<Option<Arc<TypedStore<T>>>, Busy> {
		let store = try_read(&self.0.stores)?.get(&TypeId::of::<T>()).cloned();
		Ok(store.and_then(Self::downcast))
	}

	pub(super) fn store_or_create<T: HcsgValue>(&self) -> Option<Arc<TypedStore<T>>> {
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

	pub(super) fn stamp(&self) -> u64 {
		self.0.next_version.fetch_add(1, Ordering::Relaxed) + 1
	}

	pub(super) fn base_scale_of(&self, type_id: TypeId) -> DVec3 {
		read(&self.0.base_scales).get(&type_id).copied().unwrap_or(DEFAULT_BASE_SCALE)
	}

	/// How many values of `T` are stored, including `Id::Universal`.
	pub fn len<T: HcsgValue>(&self) -> usize {
		self.store::<T>().map_or(0, |store| read(&store.nodes).len())
	}
}
