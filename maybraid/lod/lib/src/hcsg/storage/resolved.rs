//! Private resolved-store handle: locking and stored-value access for one type.
//!
//! [`HcsgStorage`] and [`GenerationContext`] both read through this handle so
//! node-store lookups stay in one place. The context caches handles for a fill;
//! it keeps reach tracking, cycle detection, and generation.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;

use crate::gen::Id;

use super::super::node_store::StoredEntry;
use super::store::{read, ErasedStore, TypedStore};
use super::{HcsgStorage, HcsgValue};

/// Cached resolved stores for one generation fill.
///
/// Each type is resolved once through the registry; later lookups reuse the
/// handle and only take the store read lock.
#[derive(Default)]
pub(in crate::hcsg) struct ResolvedStoreCache {
	stores: HashMap<TypeId, Arc<dyn ErasedStore>>,
}

impl ResolvedStoreCache {
	/// The handle for `T`, resolving and caching it on the first miss.
	pub(in crate::hcsg) fn resolve<T: HcsgValue>(
		&mut self,
		storage: &HcsgStorage,
	) -> Option<ResolvedStore<T>> {
		let type_id = TypeId::of::<T>();
		if let Some(store) = self.stores.get(&type_id) {
			return ResolvedStore::from_erased(store.clone());
		}
		let resolved = ResolvedStore::from_storage(storage)?;
		self.stores.insert(type_id, resolved.clone_erased());
		Some(resolved)
	}
}

/// One typed store after registry resolution. Owns the store read lock for
/// [`Self::get`], [`Self::contains`], and [`Self::bounds`].
pub(in crate::hcsg) struct ResolvedStore<T> {
	store: Arc<TypedStore<T>>,
}

impl<T: HcsgValue> ResolvedStore<T> {
	pub(super) fn from_storage(storage: &HcsgStorage) -> Option<Self> {
		Some(Self { store: storage.store::<T>()? })
	}

	fn from_erased(store: Arc<dyn ErasedStore>) -> Option<Self> {
		Some(Self { store: HcsgStorage::downcast(store)? })
	}

	fn clone_erased(&self) -> Arc<dyn ErasedStore> {
		self.store.clone()
	}

	pub(in crate::hcsg) fn get(&self, id: Id) -> Option<Arc<T>> {
		read(&self.store.nodes).value(id).cloned()
	}

	pub(in crate::hcsg) fn contains(&self, id: Id) -> bool {
		read(&self.store.nodes).contains(id)
	}

	pub(in crate::hcsg) fn bounds(&self, id: Id) -> Option<Aabb3d> {
		read(&self.store.nodes).entry(id).map(|entry| entry.bounds)
	}

	pub(super) fn entry(&self, id: Id) -> Option<StoredEntry<Arc<T>>> {
		read(&self.store.nodes).entry(id).cloned()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::gen::tests::test_utils::cell;

	#[test]
	fn resolved_store_reads_published_values() {
		let storage = HcsgStorage::default();
		let id = Id::from_cell(cell(3.0));
		let bounds = cell(3.0);
		storage.publish(id, Arc::new(11u32), bounds);

		let mut cache = ResolvedStoreCache::default();
		let store = cache.resolve::<u32>(&storage).expect("published type");
		assert_eq!(store.get(id).as_deref(), Some(&11));
		assert!(store.contains(id));
		assert_eq!(store.bounds(id), Some(bounds));
		assert_eq!(store.entry(id).map(|entry| *entry.value), Some(11));

		let missing = Id::from_cell(cell(99.0));
		assert!(store.get(missing).is_none());
		assert!(!store.contains(missing));
		assert!(store.bounds(missing).is_none());

		let again = cache.resolve::<u32>(&storage).expect("cached type");
		assert_eq!(again.get(id).as_deref(), Some(&11));
		assert!(cache.resolve::<u64>(&storage).is_none());
	}
}
