//! Eviction sweeps and debug rebuild tracking.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::atomic::Ordering;

use bevy::math::bounding::Aabb3d;

use crate::gen::Id;

#[cfg(debug_assertions)]
use super::store::lock_mutex;
use super::store::{read, ErasedStore};
use super::super::node_store::expand_region;
use super::HcsgStorage;

pub(super) fn record_evictions(storage: &HcsgStorage, removed: Vec<(TypeId, Id)>) {
	if removed.is_empty() {
		return;
	}
	#[cfg(debug_assertions)]
	lock_mutex(&storage.0.evicted).extend(removed);
	#[cfg(not(debug_assertions))]
	let _ = (storage, removed);
}

pub(super) fn forget_evictions_of(storage: &HcsgStorage, type_id: TypeId) {
	#[cfg(debug_assertions)]
	lock_mutex(&storage.0.evicted).retain(|(stored, _)| *stored != type_id);
	#[cfg(not(debug_assertions))]
	let _ = (storage, type_id);
}

impl HcsgStorage {
	pub fn retain_overlapping<T: super::HcsgValue>(&self, regions: &[Aabb3d]) {
		let Some(store) = self.store::<T>() else {
			return;
		};
		let removed = store.ids_outside(regions);
		if removed.is_empty() {
			return;
		}
		let stamp = self.stamp();
		store.remove_ids(&removed, stamp);
		record_evictions(self, removed.into_iter().map(|id| (TypeId::of::<T>(), id)).collect());
	}

	pub(in crate::hcsg::shared) fn retain_reached(&self, regions_by_type: &HashMap<TypeId, Vec<Aabb3d>>) {
		if regions_by_type.is_empty() {
			self.clear_evictions();
		}
		let stores: Vec<(TypeId, std::sync::Arc<dyn ErasedStore>)> = read(&self.0.stores)
			.iter()
			.map(|(type_id, store)| (*type_id, store.clone()))
			.collect();
		for (type_id, store) in stores {
			let scale = self.base_scale_of(type_id);
			let expanded: Vec<Aabb3d> = regions_by_type
				.get(&type_id)
				.into_iter()
				.flatten()
				.map(|region| expand_region(*region, scale))
				.collect();
			let removed = store.ids_outside(&expanded);
			if removed.is_empty() {
				continue;
			}
			let stamp = self.stamp();
			store.remove_ids(&removed, stamp);
			record_evictions(self, removed.into_iter().map(|id| (type_id, id)).collect());
		}
	}

	pub(in crate::hcsg::shared) fn take_evicted(&self, type_id: TypeId, id: Id) -> bool {
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

	pub(in crate::hcsg::shared) fn record_rebuild(&self, nested: bool) {
		if nested {
			self.0.nested_rebuilds.fetch_add(1, Ordering::Relaxed);
		} else {
			self.0.top_rebuilds.fetch_add(1, Ordering::Relaxed);
		}
	}

	pub fn clear_evictions(&self) {
		#[cfg(debug_assertions)]
		lock_mutex(&self.0.evicted).clear();
	}
}
