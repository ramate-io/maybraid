//! Runs schemes written against the frame-synchronous
//! [`crate::gen::GenerationScheme<S>`] on a [`GenerationContext`], so a layer
//! can move onto the shared runtime before its schemes are rewritten.
//!
//! Any type whose legacy scheme is generic over `S` (bounded only on
//! [`SpatialIndex`] and [`GeneratingSpatialIndex`]) is a [`GenerationScheme`]
//! through the blanket impl below. Legacy `descendants` hooks run only when a
//! legacy scheme generates the type; the shared runtime never calls them.
//!
//! Delete this module, and the context's [`Borrowed`] cache, once every
//! scheme is native.
//!
//! [`GeneratingSpatialIndex`]: crate::gen::GeneratingSpatialIndex

use std::any::{Any, TypeId};
use std::sync::Arc;

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use elsa::FrozenMap;

use crate::gen::{
	GenerationScheme as LegacyScheme, Id, OriginalId, SpatialIndex, StorageStatus, TrackedId,
	Version,
};

use super::context::{GenerationContext, GenerationScheme};
use super::storage::HcsgValue;

/// Values a legacy scheme has read through the context, kept alive for the
/// context's lifetime so [`SpatialIndex::get`] can lend `&T` from `&self`.
#[derive(Default)]
pub(super) struct Borrowed(FrozenMap<(TypeId, Id), Arc<dyn Any + Send + Sync>>);

impl GenerationContext<'_> {
	fn borrow<T: HcsgValue>(&self, id: Id) -> Option<&T> {
		let key = (TypeId::of::<T>(), id);
		let value = match self.borrowed.0.get(&key) {
			Some(value) => value,
			None => {
				let value: Arc<dyn Any + Send + Sync> = self.storage().get::<T>(id)?;
				self.borrowed.0.insert(key, value)
			}
		};
		value.downcast_ref()
	}

	/// Publishes and lends a value a legacy scheme inserted. Once stale,
	/// nothing is kept, so the legacy chain stops at its next read.
	fn publish_owned<T: HcsgValue>(&mut self, id: Id, value: T, bounds: Aabb3d) {
		if self.is_stale() {
			return;
		}
		let value = Arc::new(value);
		self.storage().publish(id, Arc::clone(&value), bounds);
		self.borrowed.0.as_mut().insert((TypeId::of::<T>(), id), value);
	}
}

impl<T: HcsgValue> SpatialIndex<T> for GenerationContext<'_> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.overlapping::<T>(region).into_iter().map(TrackedId).collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		let Some(entry) = self.storage().entry::<T>(id) else {
			return StorageStatus::NotTracked;
		};
		match id.origin_cell_bounds() {
			Some(origin) if !origin.intersects(&entry.bounds) => StorageStatus::TrackedOutside,
			_ => StorageStatus::TrackedWithin,
		}
	}

	fn get(&self, id: Id) -> Option<&T> {
		self.borrow(id)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.storage().entry::<T>(id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.storage().entry::<T>(id).map(|entry| entry.version)
	}

	fn membership_revision(&self) -> u64 {
		self.storage().membership_revision::<T>()
	}

	fn insert(&mut self, id: Id, value: T, bounds: Aabb3d) {
		self.publish_owned(id, value, bounds);
	}
}

impl<T> GenerationScheme for T
where
	T: HcsgValue + for<'a> LegacyScheme<GenerationContext<'a>>,
{
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		<T as LegacyScheme<GenerationContext>>::original_ids_for(cx, region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		<T as LegacyScheme<GenerationContext>>::build_with_id(cx, id)
	}
}
