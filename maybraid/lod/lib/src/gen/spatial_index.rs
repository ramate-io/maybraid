use crate::gen::id::{Id, OriginalId, StorageStatus, TrackedId};
use crate::lod_ref::LodRef;
use bevy::math::bounding::Aabb3d;
use std::collections::HashSet;

pub trait BaseSpatialIndex<T> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId>;

	fn storage_status(&self, id: Id) -> StorageStatus;

	fn get(&self, id: Id) -> Option<&T>;

	fn get_bounds(&self, id: Id) -> Option<Aabb3d>;

	fn get_with_bounds(&self, id: Id) -> Option<(&T, Aabb3d)> {
		self.get(id).and_then(|t| self.get_bounds(id).map(|b| (t, b)))
	}

	/// Inserts the type into the spatial index. Must not spawn scenes.
	fn insert(&mut self, id: Id, t: T, bounds: Aabb3d, lod_ref: &LodRef);
}

pub trait SpatialIndex<T>: BaseSpatialIndex<T> {
	fn original_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId>;

	fn all_ids_for(&mut self, region: Aabb3d) -> Vec<Id> {
		self.original_ids_for(region)
			.into_iter()
			.map(|id| id.0)
			.chain(self.tracked_ids_for(region).into_iter().map(|id| id.0))
			.collect()
	}

	fn fresh_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId> {
		self.original_ids_for(region)
			.into_iter()
			.filter(|id| self.storage_status(id.0) == StorageStatus::NotTracked)
			.collect()
	}

	fn ids_for(&mut self, region: Aabb3d) -> Vec<Id> {
		self.fresh_ids_for(region)
			.into_iter()
			.map(|id| id.0)
			.chain(self.tracked_ids_for(region).into_iter().map(|id| id.0))
			.collect()
	}

	fn deduplicated_ids_for(&mut self, region: Aabb3d) -> Vec<Id> {
		self.ids_for(region).into_iter().collect::<HashSet<_>>().into_iter().collect()
	}
}

pub trait OriginalIds<S> {
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId>;
}

impl<T, S> SpatialIndex<T> for S
where
	S: BaseSpatialIndex<T>,
	T: OriginalIds<S>,
{
	fn original_ids_for(&mut self, region: Aabb3d) -> Vec<OriginalId> {
		T::original_ids_for(self, region)
	}
}
