//! [`GenerationScheme`]s and the [`GenerationContext`] they generate through.

use std::any::TypeId;
use std::collections::HashSet;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;

use crate::gen::{Id, OriginalId};

use super::adapter::Borrowed;
use super::storage::{HcsgStorage, HcsgValue};

/// How one generated type is discovered and built.
///
/// A value is a pure function of its key `(TypeId, Id)` and the session roots.
/// Schemes read only through the [`GenerationContext`].
pub trait GenerationScheme: HcsgValue + Sized {
	/// Ids that originate in `region`. May generate dependencies through `cx`.
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId>;

	/// The value for `id` and its bounds, or `None` where nothing exists.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)>;
}

/// Storage access for schemes: owned `Arc` dependencies, generated on demand.
///
/// Locks are taken only to look up and publish, so a dependency stays usable
/// after its store is released, and recursion never holds a lock.
pub struct GenerationContext<'a> {
	storage: &'a HcsgStorage,
	/// `(TypeId, Id)` pairs being built on this stack; a repeat is a cycle.
	generating: HashSet<(TypeId, Id)>,
	stale: &'a (dyn Fn() -> bool + 'a),
	pub(super) borrowed: Borrowed,
}

impl<'a> GenerationContext<'a> {
	pub fn new(storage: &'a HcsgStorage) -> Self {
		Self::with_stale(storage, &|| false)
	}

	/// Once `stale` returns true, nothing more is generated or published.
	pub fn with_stale(storage: &'a HcsgStorage, stale: &'a (dyn Fn() -> bool + 'a)) -> Self {
		Self { storage, generating: HashSet::new(), stale, borrowed: Borrowed::default() }
	}

	pub fn storage(&self) -> &HcsgStorage {
		self.storage
	}

	pub fn is_stale(&self) -> bool {
		(self.stale)()
	}

	pub fn get<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>> {
		self.storage.get(id)
	}

	pub fn overlapping<T: HcsgValue>(&self, region: Aabb3d) -> Vec<Id> {
		self.storage.overlapping::<T>(region)
	}

	pub fn original_ids_for<T: GenerationScheme>(&mut self, region: Aabb3d) -> Vec<OriginalId> {
		T::original_ids_for(self, region)
	}

	/// The stored value, else builds and publishes it, resolving its own
	/// dependencies on the way. `None` where nothing exists, on a cycle, or
	/// once stale.
	pub fn get_or_generate<T: GenerationScheme>(&mut self, id: Id) -> Option<Arc<T>> {
		if let Some(value) = self.storage.get::<T>(id) {
			return Some(value);
		}
		let key = (TypeId::of::<T>(), id);
		if self.is_stale() || !self.generating.insert(key) {
			return None;
		}
		let built = T::build_with_id(self, id);
		self.generating.remove(&key);
		let (value, bounds) = built?;
		if self.is_stale() {
			return None;
		}
		let value = Arc::new(value);
		self.storage.publish(id, Arc::clone(&value), bounds);
		Some(value)
	}
}
