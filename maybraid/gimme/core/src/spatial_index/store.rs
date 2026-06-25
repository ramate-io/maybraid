//! Pluggable typed value storage for [`TypedIndex`](super::typed::TypedIndex).

use std::collections::HashMap;

use bevy_math::bounding::Aabb3d;

use crate::error::SpatialIndexError;

/// Stable spatial identity carried by indexed values.
pub trait SpatialId {
	type Id: Copy + Eq + std::hash::Hash + Ord;

	fn spatial_id(&self) -> Self::Id;
}

/// Maps typed payloads (by their own id) to bounds.
pub trait SpatialStore<T>
where
	T: SpatialId,
{
	fn get(&self, id: T::Id) -> Option<&T>;

	fn bounds(&self, id: T::Id) -> Option<Aabb3d>;

	fn insert(&mut self, value: T, bounds: Aabb3d) -> Result<(), SpatialIndexError>;

	fn remove(&mut self, id: T::Id) -> Option<T>;

	fn iter<'a>(&'a self) -> impl Iterator<Item = (T::Id, &'a T, Aabb3d)> + 'a
	where
		T: 'a;
}

/// [`HashMap`] backend for tests and offline tools.
#[derive(Debug)]
pub struct HashMapStore<T: SpatialId> {
	values: HashMap<T::Id, T>,
	bounds: HashMap<T::Id, Aabb3d>,
}

impl<T: SpatialId> Default for HashMapStore<T> {
	fn default() -> Self {
		Self { values: HashMap::new(), bounds: HashMap::new() }
	}
}

impl<T: SpatialId> HashMapStore<T> {
	pub fn new() -> Self {
		Self::default()
	}
}

impl<T: SpatialId> SpatialStore<T> for HashMapStore<T> {
	fn get(&self, id: T::Id) -> Option<&T> {
		self.values.get(&id)
	}

	fn bounds(&self, id: T::Id) -> Option<Aabb3d> {
		self.bounds.get(&id).copied()
	}

	fn insert(&mut self, value: T, bounds: Aabb3d) -> Result<(), SpatialIndexError> {
		let id = value.spatial_id();
		self.values.insert(id, value);
		self.bounds.insert(id, bounds);
		Ok(())
	}

	fn remove(&mut self, id: T::Id) -> Option<T> {
		self.bounds.remove(&id);
		self.values.remove(&id)
	}

	fn iter<'a>(&'a self) -> impl Iterator<Item = (T::Id, &'a T, Aabb3d)> + 'a
	where
		T: 'a,
	{
		let mut ids: Vec<_> = self.values.keys().copied().collect();
		ids.sort();
		ids.into_iter().filter_map(|id| {
			let value = self.values.get(&id)?;
			let bounds = self.bounds.get(&id).copied()?;
			Some((id, value, bounds))
		})
	}
}
