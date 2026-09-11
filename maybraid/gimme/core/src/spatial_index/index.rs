//! Type-agnostic multi-resolution spatial index ([RFC-142 §3.1]).
//!
//! The only indexed shape is an AABB. Each object is stored in the canonical
//! cells it intersects at the coarsest level that can bound it, so a large
//! AABB does not fan out across a fine grid.

use std::collections::{HashMap, HashSet};

use bevy_math::bounding::{Aabb3d, IntersectsVolume};
use bevy_math::{DVec3, IVec3};

use crate::error::SpatialIndexError;

use super::grid::{BaseScale, Level};

type GridKey = (Level, IVec3);

/// Implicit multi-resolution grid keyed by `(Level, IVec3)` slots.
#[derive(Debug, Clone)]
pub struct SpatialIndex<Id> {
	base_scale: BaseScale,
	cells: HashMap<GridKey, HashSet<Id>>,
	bounds: HashMap<Id, Aabb3d>,
	id_cells: HashMap<Id, Vec<GridKey>>,
	max_level: Level,
}

impl<Id> SpatialIndex<Id>
where
	Id: Copy + Eq + std::hash::Hash + Ord,
{
	pub fn new(base_scale: DVec3) -> Result<Self, SpatialIndexError> {
		Ok(Self {
			base_scale: BaseScale::new(base_scale)?,
			cells: HashMap::new(),
			bounds: HashMap::new(),
			id_cells: HashMap::new(),
			max_level: 0,
		})
	}

	pub fn base_scale(&self) -> DVec3 {
		self.base_scale.as_dvec3()
	}

	pub fn len(&self) -> usize {
		self.bounds.len()
	}

	pub fn is_empty(&self) -> bool {
		self.bounds.is_empty()
	}

	pub fn cell_count(&self) -> usize {
		self.cells.len()
	}

	pub fn max_level(&self) -> Level {
		self.max_level
	}

	pub fn get(&self, id: Id) -> Option<Aabb3d> {
		self.bounds.get(&id).copied()
	}

	fn intersecting_cells(&self, bounds: &Aabb3d) -> Vec<GridKey> {
		let level = self.base_scale.insertion_level(bounds);
		let mut keys = Vec::new();
		self.base_scale.for_each_coord(bounds, level, |coord| keys.push((level, coord)));
		keys
	}

	pub fn insert(&mut self, id: Id, bounds: Aabb3d) -> Result<(), SpatialIndexError> {
		if self.bounds.contains_key(&id) {
			self.remove(id);
		}

		let keys = self.intersecting_cells(&bounds);
		self.max_level = self.max_level.max(keys.first().map(|(level, _)| *level).unwrap_or(0));
		for key in &keys {
			self.cells.entry(*key).or_default().insert(id);
		}

		self.bounds.insert(id, bounds);
		self.id_cells.insert(id, keys);
		Ok(())
	}

	pub fn remove(&mut self, id: Id) -> Option<Aabb3d> {
		let keys = self.id_cells.remove(&id)?;
		for key in keys {
			if let Some(bucket) = self.cells.get_mut(&key) {
				bucket.remove(&id);
				if bucket.is_empty() {
					self.cells.remove(&key);
				}
			}
		}
		self.bounds.remove(&id)
	}

	pub fn iter_all(&self) -> impl Iterator<Item = (Id, Aabb3d)> + '_ {
		let mut ids: Vec<_> = self.bounds.keys().copied().collect();
		ids.sort();
		ids.into_iter()
			.filter_map(|id| self.bounds.get(&id).copied().map(|bounds| (id, bounds)))
	}

	/// All objects whose AABB intersects `region`, including larger objects
	/// stored at coarser levels than the query itself.
	pub fn query(&self, region: Aabb3d) -> Vec<Id> {
		self.query_ids(region, BaseScale::levels_through(self.max_level))
	}

	pub fn query_iter(
		&self,
		region: Aabb3d,
		levels: impl IntoIterator<Item = Level>,
	) -> impl Iterator<Item = (Id, Aabb3d)> + '_ {
		self.query_pairs(region, levels).into_iter()
	}

	pub fn sub_index(
		&self,
		region: Aabb3d,
		levels: impl IntoIterator<Item = Level>,
	) -> Result<SpatialIndex<Id>, SpatialIndexError> {
		let mut sub = SpatialIndex::new(self.base_scale.as_dvec3())?;
		for (id, bounds) in self.query_pairs(region, levels) {
			sub.insert(id, bounds)?;
		}
		Ok(sub)
	}

	fn query_ids(&self, region: Aabb3d, levels: impl IntoIterator<Item = Level>) -> Vec<Id> {
		self.query_pairs(region, levels).into_iter().map(|(id, _)| id).collect()
	}

	fn query_pairs(
		&self,
		region: Aabb3d,
		levels: impl IntoIterator<Item = Level>,
	) -> Vec<(Id, Aabb3d)> {
		let mut seen = HashSet::new();
		let mut result = Vec::new();

		for level in levels {
			self.base_scale.for_each_coord(&region, level, |coord| {
				let Some(bucket) = self.cells.get(&(level, coord)) else {
					return;
				};
				for &id in bucket {
					if !seen.insert(id) {
						continue;
					}
					let Some(&bounds) = self.bounds.get(&id) else {
						continue;
					};
					if bounds.intersects(&region) {
						result.push((id, bounds));
					}
				}
			});
		}

		result.sort_by_key(|(id, _)| *id);
		result
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;

	fn index() -> Result<SpatialIndex<u32>> {
		Ok(SpatialIndex::new(DVec3::ONE)?)
	}

	fn aabb(min: [f32; 3], max: [f32; 3]) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::from_array(min), Vec3::from_array(max))
	}

	#[test]
	fn insertion_level_boundaries() -> Result<()> {
		let mut idx = index()?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))?;
		idx.insert(2, aabb([0.0, 0.0, 0.0], [2.0, 1.0, 1.0]))?;
		assert_eq!(idx.len(), 2);
		Ok(())
	}

	#[test]
	fn large_object_uses_coarse_cells() -> Result<()> {
		let mut idx = index()?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [2.0, 1.0, 1.0]))?;
		assert_eq!(idx.cell_count(), 2);
		assert_eq!(idx.max_level(), 1);
		Ok(())
	}

	#[test]
	fn query_finds_coarse_hosts_from_a_small_region() -> Result<()> {
		let mut idx = index()?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [4.0, 4.0, 4.0]))?;
		let hits = idx.query(aabb([0.1, 0.1, 0.1], [0.2, 0.2, 0.2]));
		assert_eq!(hits, vec![1]);
		Ok(())
	}

	#[test]
	fn query_deduplicates_multi_cell_object() -> Result<()> {
		let mut idx = index()?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [2.0, 1.0, 1.0]))?;
		let hits: Vec<_> = idx.query_iter(aabb([0.0, 0.0, 0.0], [2.0, 1.0, 1.0]), [1]).collect();
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].0, 1);
		Ok(())
	}

	#[test]
	fn exact_aabb_filter_excludes_non_intersecting() -> Result<()> {
		let mut idx = index()?;
		idx.insert(1, aabb([1.0, 0.0, 0.0], [1.1, 1.0, 1.0]))?;
		let hits: Vec<_> = idx.query_iter(aabb([0.0, 0.0, 0.0], [0.5, 1.0, 1.0]), [0]).collect();
		assert!(hits.is_empty());
		Ok(())
	}

	#[test]
	fn query_across_levels() -> Result<()> {
		let mut idx = index()?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))?;
		idx.insert(2, aabb([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]))?;
		let hits: Vec<_> = idx.query_iter(aabb([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]), [0, 1]).collect();
		assert_eq!(hits.len(), 2);
		Ok(())
	}

	#[test]
	fn deterministic_replay() -> Result<()> {
		let mut idx = index()?;
		idx.insert(3, aabb([1.0, 0.0, 0.0], [2.0, 1.0, 1.0]))?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))?;
		idx.insert(2, aabb([0.5, 0.0, 0.0], [1.5, 1.0, 1.0]))?;
		let region = aabb([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
		let a: Vec<_> = idx.query_iter(region, [0]).collect();
		let b: Vec<_> = idx.query_iter(region, [0]).collect();
		assert_eq!(a, b);
		assert_eq!(a.iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![1, 2, 3]);
		Ok(())
	}

	#[test]
	fn sub_index_materializes_subset() -> Result<()> {
		let mut idx = index()?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))?;
		idx.insert(2, aabb([5.0, 0.0, 0.0], [6.0, 1.0, 1.0]))?;
		let sub = idx.sub_index(aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]), [0])?;
		assert_eq!(sub.len(), 1);
		assert_eq!(sub.get(1), Some(aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])));
		assert_eq!(sub.get(2), None);
		Ok(())
	}

	#[test]
	fn update_moves_buckets() -> Result<()> {
		let mut idx = index()?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))?;
		idx.insert(1, aabb([5.0, 0.0, 0.0], [6.0, 1.0, 1.0]))?;
		let near: Vec<_> = idx.query_iter(aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]), [0]).collect();
		let far: Vec<_> = idx.query_iter(aabb([5.0, 0.0, 0.0], [6.0, 1.0, 1.0]), [0]).collect();
		assert!(near.is_empty());
		assert_eq!(far.len(), 1);
		Ok(())
	}

	#[test]
	fn remove_clears_buckets() -> Result<()> {
		let mut idx = index()?;
		idx.insert(1, aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]))?;
		assert_eq!(idx.remove(1), Some(aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])));
		assert!(idx.query(aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])).is_empty());
		assert_eq!(idx.cell_count(), 0);
		Ok(())
	}
}
