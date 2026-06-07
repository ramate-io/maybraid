//! Typed spatial index composing [`SpatialIndex`](super::SpatialIndex) over [`SpatialStore`](super::store::SpatialStore).

use bevy_math::bounding::Aabb3d;
use bevy_math::DVec3;

use crate::error::SpatialIndexError;
use crate::spatial_index::grid::{BaseScale, Level};
use crate::spatial_index::index::SpatialIndex;
use crate::spatial_index::store::{SpatialId, SpatialStore};
use crate::TypedSpatialIndex;

/// Grid index plus pluggable typed value storage (mirrors the `TypedBucketThrow` composition pattern).
pub struct TypedIndex<T, S>
where
	T: SpatialId,
	S: SpatialStore<T>,
{
	grid: SpatialIndex<T::Id>,
	store: S,
}

impl<T, S> TypedIndex<T, S>
where
	T: SpatialId,
	S: SpatialStore<T>,
{
	pub fn new(base_scale: DVec3, store: S) -> Result<Self, SpatialIndexError> {
		Ok(Self { grid: SpatialIndex::new(base_scale)?, store })
	}

	pub fn grid(&self) -> &SpatialIndex<T::Id> {
		&self.grid
	}

	pub fn grid_mut(&mut self) -> &mut SpatialIndex<T::Id> {
		&mut self.grid
	}

	pub fn store(&self) -> &S {
		&self.store
	}

	pub fn store_mut(&mut self) -> &mut S {
		&mut self.store
	}

	pub fn insert_value(&mut self, value: T, bounds: Aabb3d) -> Result<T::Id, SpatialIndexError> {
		let id = value.spatial_id();
		self.store.insert(value, bounds)?;
		self.grid.insert(id, bounds)?;
		Ok(id)
	}

	pub fn remove_value(&mut self, id: T::Id) -> Option<T> {
		self.grid.remove(id);
		self.store.remove(id)
	}

	pub fn query_values(
		&self,
		region: Aabb3d,
		levels: impl IntoIterator<Item = Level>,
	) -> impl Iterator<Item = (&T, Aabb3d)> + '_ {
		self.grid.query_iter(region, levels).filter_map(|(id, bounds)| {
			self.store.get(id).map(|value| (value, bounds))
		})
	}
}

impl<T, S> TypedSpatialIndex<T> for TypedIndex<T, S>
where
	T: SpatialId,
	S: SpatialStore<T>,
{
	fn read_one(&self, region: &Aabb3d) -> Result<Option<&T>, SpatialIndexError> {
		let base = BaseScale::new(self.grid.base_scale())?;
		let levels: Vec<_> = base.levels_for_bounds(region).collect();
		for (id, bounds) in self.grid.query_iter(*region, levels) {
			if bounds == *region {
				return Ok(self.store.get(id));
			}
		}
		Ok(None)
	}

	fn insert(&mut self, value: T, bounds: Aabb3d) -> Result<&T, SpatialIndexError> {
		let id = value.spatial_id();
		self.insert_value(value, bounds)?;
		self.store.get(id).ok_or(SpatialIndexError::InsertFailed)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::spatial_index::store::HashMapStore;
	use anyhow::Result;
	use bevy_math::Vec3;

	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
	struct Slot(u32);

	impl SpatialId for Slot {
		type Id = u32;

		fn spatial_id(&self) -> Self::Id {
			self.0
		}
	}

	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
	struct Named {
		id: u32,
		label: &'static str,
	}

	impl SpatialId for Named {
		type Id = u32;

		fn spatial_id(&self) -> Self::Id {
			self.id
		}
	}

	fn aabb(min: [f32; 3], max: [f32; 3]) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::from_array(min), Vec3::from_array(max))
	}

	#[test]
	fn typed_insert_and_read_one() -> Result<()> {
		let mut idx = TypedIndex::new(DVec3::ONE, HashMapStore::new())?;
		let region = aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
		idx.insert(Slot(42), region)?;
		assert_eq!(idx.read_one(&region)?, Some(&Slot(42)));
		Ok(())
	}

	#[test]
	fn typed_query_returns_values() -> Result<()> {
		let mut idx = TypedIndex::new(DVec3::ONE, HashMapStore::new())?;
		idx.insert(Named { id: 1, label: "a" }, aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))?;
		idx.insert(Named { id: 2, label: "b" }, aabb([5.0, 0.0, 0.0], [6.0, 1.0, 1.0]))?;
		let hits: Vec<_> = idx
			.query_values(aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]), [0])
			.collect();
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].0.label, "a");
		Ok(())
	}
}
