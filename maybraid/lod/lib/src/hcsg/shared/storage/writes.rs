//! Publication, removal, and session clears.

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;

use crate::gen::{Id, Version};

use super::eviction::forget_evictions_of;
use super::store::{read, write};
use super::{HcsgStorage, HcsgValue};
use super::super::node_store::StoredEntry;

impl HcsgStorage {
	pub fn publish<T: HcsgValue>(&self, id: Id, value: Arc<T>, bounds: Aabb3d) -> Version {
		let stamp = self.stamp();
		let version = Version(stamp);
		if let Some(store) = self.store_or_create::<T>() {
			write(&store.nodes).put(id, StoredEntry { value, bounds, version }, stamp);
		}
		version
	}

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

	pub fn seed<T: HcsgValue>(&self, value: T, bounds: Aabb3d) -> Version {
		self.publish(Id::Universal, Arc::new(value), bounds)
	}

	pub fn remove<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>> {
		let stamp = self.stamp();
		let store = self.store::<T>()?;
		let removed = write(&store.nodes).remove(id, stamp).map(|entry| entry.value);
		removed
	}

	pub fn clear<T: HcsgValue>(&self) {
		let stamp = self.stamp();
		let store = read(&self.0.stores).get(&std::any::TypeId::of::<T>()).cloned();
		if let Some(store) = store {
			store.reset(stamp);
		}
		forget_evictions_of(self, std::any::TypeId::of::<T>());
	}
}
