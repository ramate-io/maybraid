//! Typed store registry, per-type [`NodeStore`] handles, and index config.

use std::any::{Any, TypeId};
use std::collections::HashMap;
#[cfg(debug_assertions)]
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};
#[cfg(debug_assertions)]
use std::sync::{Mutex, MutexGuard};

use bevy::math::DVec3;
use bevy::shape::Aabb3d;

use crate::gen::Id;

use super::super::context::GenerationScheme;
use super::super::node_store::{NodeStore, DEFAULT_BASE_SCALE};
use super::{Busy, HcsgStorage, HcsgValue};

#[cfg(feature = "generation-tracing")]
type ReadGuard<'a, T> = lod_generation_diagnostics::MeasuredGuard<RwLockReadGuard<'a, T>>;
#[cfg(not(feature = "generation-tracing"))]
type ReadGuard<'a, T> = RwLockReadGuard<'a, T>;
#[cfg(feature = "generation-tracing")]
type WriteGuard<'a, T> = lod_generation_diagnostics::MeasuredGuard<RwLockWriteGuard<'a, T>>;
#[cfg(not(feature = "generation-tracing"))]
type WriteGuard<'a, T> = RwLockWriteGuard<'a, T>;

fn read_lock<'a, T>(lock: &'a RwLock<T>, _kind: &'static str) -> ReadGuard<'a, T> {
	#[cfg(feature = "generation-tracing")]
	let wait = lod_generation_diagnostics::LockWait::new(_kind);
	let guard = lock.read().unwrap_or_else(PoisonError::into_inner);
	#[cfg(feature = "generation-tracing")]
	{
		wait.acquired(guard)
	}
	#[cfg(not(feature = "generation-tracing"))]
	{
		guard
	}
}
fn write_lock<'a, T>(lock: &'a RwLock<T>, _kind: &'static str) -> WriteGuard<'a, T> {
	#[cfg(feature = "generation-tracing")]
	let wait = lod_generation_diagnostics::LockWait::new(_kind);
	let guard = lock.write().unwrap_or_else(PoisonError::into_inner);
	#[cfg(feature = "generation-tracing")]
	{
		wait.acquired(guard)
	}
	#[cfg(not(feature = "generation-tracing"))]
	{
		guard
	}
}
fn try_read_lock<'a, T>(
	lock: &'a RwLock<T>,
	_kind: &'static str,
) -> Result<ReadGuard<'a, T>, Busy> {
	#[cfg(feature = "generation-tracing")]
	let wait = lod_generation_diagnostics::LockWait::new(_kind);
	let guard = match lock.try_read() {
		Ok(guard) => guard,
		Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
		Err(TryLockError::WouldBlock) => {
			#[cfg(feature = "generation-tracing")]
			lod_generation_diagnostics::update(|c| c.busy += 1);
			return Err(Busy);
		}
	};
	#[cfg(feature = "generation-tracing")]
	{
		Ok(wait.acquired(guard))
	}
	#[cfg(not(feature = "generation-tracing"))]
	{
		Ok(guard)
	}
}

#[derive(Default)]
pub(super) struct Registry {
	pub stores: RwLock<HashMap<TypeId, Arc<dyn ErasedStore>>>,
	pub base_scales: RwLock<HashMap<TypeId, DVec3>>,
	pub retention_margins: RwLock<HashMap<TypeId, DVec3>>,
	pub next_version: AtomicU64,
	/// Keys removed by a sweep; debug rebuilds check this set.
	#[cfg(debug_assertions)]
	pub evicted: Mutex<HashSet<(TypeId, Id)>>,
	pub top_rebuilds: AtomicU64,
	pub nested_rebuilds: AtomicU64,
}

impl Registry {
	pub(super) fn read_stores(&self) -> ReadGuard<'_, HashMap<TypeId, Arc<dyn ErasedStore>>> {
		read_lock(&self.stores, "registry")
	}

	pub(super) fn write_stores(&self) -> WriteGuard<'_, HashMap<TypeId, Arc<dyn ErasedStore>>> {
		write_lock(&self.stores, "registry")
	}

	pub(super) fn try_read_stores(
		&self,
	) -> Result<ReadGuard<'_, HashMap<TypeId, Arc<dyn ErasedStore>>>, Busy> {
		try_read_lock(&self.stores, "registry")
	}

	pub(super) fn read_base_scales(&self) -> ReadGuard<'_, HashMap<TypeId, DVec3>> {
		read_lock(&self.base_scales, "registry")
	}

	pub(super) fn write_base_scales(&self) -> WriteGuard<'_, HashMap<TypeId, DVec3>> {
		write_lock(&self.base_scales, "registry")
	}

	pub(super) fn read_retention_margins(&self) -> ReadGuard<'_, HashMap<TypeId, DVec3>> {
		read_lock(&self.retention_margins, "registry")
	}

	pub(super) fn write_retention_margins(&self) -> WriteGuard<'_, HashMap<TypeId, DVec3>> {
		write_lock(&self.retention_margins, "registry")
	}

	#[cfg(debug_assertions)]
	pub(super) fn lock_evicted(&self) -> MutexGuard<'_, HashSet<(TypeId, Id)>> {
		self.evicted.lock().unwrap_or_else(PoisonError::into_inner)
	}
}

pub(in crate::hcsg) struct TypedStore<T> {
	nodes: RwLock<NodeStore<Arc<T>>>,
}

impl<T> TypedStore<T> {
	pub(in crate::hcsg) fn new(base_scale: DVec3) -> Self {
		Self { nodes: RwLock::new(NodeStore::new(base_scale)) }
	}

	/// Values are immutable and published whole, so a poisoned lock still guards
	/// a consistent store.
	pub(in crate::hcsg) fn read(&self) -> ReadGuard<'_, NodeStore<Arc<T>>> {
		read_lock(&self.nodes, "store")
	}

	/// Values are immutable and published whole, so a poisoned lock still guards
	/// a consistent store.
	pub(in crate::hcsg) fn write(&self) -> WriteGuard<'_, NodeStore<Arc<T>>> {
		write_lock(&self.nodes, "store")
	}

	pub(in crate::hcsg) fn try_read(&self) -> Result<ReadGuard<'_, NodeStore<Arc<T>>>, Busy> {
		try_read_lock(&self.nodes, "store")
	}
}

pub(in crate::hcsg) trait ErasedStore: Send + Sync {
	fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
	fn reset(&self, revision: u64);
	fn evict_outside(&self, regions: &[Aabb3d], revision: u64) -> Vec<Id>;
	/// Drops every entry except [`Id::Universal`]. Returns removed ids only
	/// under `cfg(debug_assertions)`; release builds clear without listing them.
	fn clear_non_universal(&self, revision: u64) -> Vec<Id>;
	fn len(&self) -> usize;
	fn type_name(&self) -> &'static str;
}

impl<T: HcsgValue> ErasedStore for TypedStore<T> {
	fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
		self
	}

	fn reset(&self, revision: u64) {
		let mut nodes = self.write();
		#[cfg(feature = "generation-tracing")]
		let before = nodes.len();
		nodes.reset(revision);
		#[cfg(feature = "generation-tracing")]
		lod_generation_diagnostics::update(|c| {
			c.removed += before.saturating_sub(nodes.len()) as u64
		});
	}

	fn evict_outside(&self, regions: &[Aabb3d], revision: u64) -> Vec<Id> {
		{
			let mut nodes = self.write();
			#[cfg(feature = "generation-tracing")]
			let before = nodes.len();
			let removed = nodes.evict_outside(regions, revision);
			#[cfg(feature = "generation-tracing")]
			lod_generation_diagnostics::update(|c| {
				c.removed += before.saturating_sub(nodes.len()) as u64
			});
			removed
		}
	}

	fn clear_non_universal(&self, revision: u64) -> Vec<Id> {
		{
			let mut nodes = self.write();
			#[cfg(feature = "generation-tracing")]
			let before = nodes.len();
			let removed = nodes.clear_non_universal(revision);
			#[cfg(feature = "generation-tracing")]
			lod_generation_diagnostics::update(|c| {
				c.removed += before.saturating_sub(nodes.len()) as u64
			});
			removed
		}
	}

	fn len(&self) -> usize {
		self.read().len()
	}

	fn type_name(&self) -> &'static str {
		std::any::type_name::<T>()
	}
}

impl HcsgStorage {
	/// Records `T`'s index scale and retention margin before its first publish.
	/// A scale set earlier through [`Self::configure`] wins.
	pub(crate) fn register_scheme<T: GenerationScheme>(&self) {
		let type_id = TypeId::of::<T>();
		if !self.0.read_base_scales().contains_key(&type_id) {
			self.0.write_base_scales().entry(type_id).or_insert(T::INDEX_SCALE);
		}
		if !self.0.read_retention_margins().contains_key(&type_id) {
			self.0.write_retention_margins().entry(type_id).or_insert(T::RETENTION_MARGIN);
		}
	}

	/// Sets `T`'s spatial base scale, rebuilding its index if it exists.
	pub fn configure<T: HcsgValue>(&self, base_scale: DVec3) -> &Self {
		self.0.write_base_scales().insert(TypeId::of::<T>(), base_scale);
		if let Some(store) = self.store::<T>() {
			store.write().rebuild_index(base_scale);
		}
		self
	}

	pub(super) fn downcast<T: HcsgValue>(
		store: Arc<dyn ErasedStore>,
	) -> Option<Arc<TypedStore<T>>> {
		store.into_any().downcast().ok()
	}

	pub(in crate::hcsg) fn store<T: HcsgValue>(&self) -> Option<Arc<TypedStore<T>>> {
		let store = self.0.read_stores().get(&TypeId::of::<T>()).cloned()?;
		Self::downcast(store)
	}

	pub(super) fn try_store<T: HcsgValue>(&self) -> Result<Option<Arc<TypedStore<T>>>, Busy> {
		let store = self.0.try_read_stores()?.get(&TypeId::of::<T>()).cloned();
		Ok(store.and_then(Self::downcast))
	}

	pub(super) fn store_or_create<T: HcsgValue>(&self) -> Option<Arc<TypedStore<T>>> {
		if let Some(store) = self.store::<T>() {
			return Some(store);
		}
		let base_scale = self.base_scale_of(TypeId::of::<T>());
		let store = self
			.0
			.write_stores()
			.entry(TypeId::of::<T>())
			.or_insert_with(|| Arc::new(TypedStore::<T>::new(base_scale)))
			.clone();
		Self::downcast(store)
	}

	pub(super) fn stamp(&self) -> u64 {
		self.0.next_version.fetch_add(1, Ordering::Relaxed) + 1
	}

	pub(super) fn base_scale_of(&self, type_id: TypeId) -> DVec3 {
		self.0.read_base_scales().get(&type_id).copied().unwrap_or(DEFAULT_BASE_SCALE)
	}

	pub(super) fn retention_margin_of(&self, type_id: TypeId) -> DVec3 {
		self.0
			.read_retention_margins()
			.get(&type_id)
			.copied()
			.unwrap_or_else(|| self.base_scale_of(type_id))
	}

	#[cfg(test)]
	pub(crate) fn index_scale_of<T: HcsgValue>(&self) -> DVec3 {
		self.base_scale_of(TypeId::of::<T>())
	}

	/// How many values of `T` are stored, including `Id::Universal`.
	pub fn len<T: HcsgValue>(&self) -> usize {
		self.store::<T>().map_or(0, |store| store.read().len())
	}
}
