//! Typed HCSG storage: one homogeneous [`NodeStore`] per generated type.

use std::collections::{HashMap, HashSet};

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{DVec3, Vec3};

use crate::gen::{Id, Version};

/// Base cell size of a store's spatial index when none is configured.
pub const DEFAULT_BASE_SCALE: DVec3 = DVec3::splat(64.0);

/// One stored value with its authoritative bounds and storage version.
#[derive(Debug, Clone)]
pub struct StoredEntry<T> {
	pub value: T,
	pub bounds: Aabb3d,
	pub version: Version,
}

/// Values of one type, their bounds, and a spatial index over those bounds.
///
/// Every mutation goes through [`HcsgStorage`], which keeps `entries`, the
/// spatial index, and the membership revision in step. `Id::Universal`
/// entries are reachable by id only; they are never spatially indexed.
pub struct NodeStore<T> {
	entries: HashMap<Id, StoredEntry<T>>,
	/// `None` when gimme rejects the base scale; queries then walk `entries`.
	spatial: Option<gimme_core::SpatialIndex<Id>>,
	/// Ids gimme would not index (non-finite bounds). Every query walks them.
	unindexed: HashSet<Id>,
	base_scale: DVec3,
	membership_revision: u64,
}

impl<T> NodeStore<T> {
	pub(super) fn new(base_scale: DVec3) -> Self {
		Self {
			entries: HashMap::new(),
			spatial: gimme_core::SpatialIndex::new(base_scale).ok(),
			unindexed: HashSet::new(),
			base_scale,
			membership_revision: 0,
		}
	}

	pub(super) fn rebuild_index(&mut self, base_scale: DVec3) {
		self.base_scale = base_scale;
		self.spatial = gimme_core::SpatialIndex::new(base_scale).ok();
		self.unindexed.clear();
		let entries: Vec<(Id, Aabb3d)> =
			self.entries.iter().map(|(id, entry)| (*id, entry.bounds)).collect();
		for (id, bounds) in entries {
			self.index(id, bounds);
		}
	}

	fn index(&mut self, id: Id, bounds: Aabb3d) {
		if id == Id::Universal {
			return;
		}
		let indexed =
			self.spatial.as_mut().is_some_and(|spatial| spatial.insert(id, bounds).is_ok());
		if indexed {
			self.unindexed.remove(&id);
		} else {
			if let Some(spatial) = self.spatial.as_mut() {
				spatial.remove(id);
			}
			self.unindexed.insert(id);
		}
	}

	pub fn entry(&self, id: Id) -> Option<&StoredEntry<T>> {
		self.entries.get(&id)
	}

	pub fn value(&self, id: Id) -> Option<&T> {
		self.entries.get(&id).map(|entry| &entry.value)
	}

	pub fn contains(&self, id: Id) -> bool {
		self.entries.contains_key(&id)
	}

	pub fn len(&self) -> usize {
		self.entries.len()
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}

	pub fn iter(&self) -> impl Iterator<Item = (Id, &StoredEntry<T>)> {
		self.entries.iter().map(|(id, entry)| (*id, entry))
	}

	/// Monotonic stamp bumped by every insert, removal, and clear.
	pub fn membership_revision(&self) -> u64 {
		self.membership_revision
	}

	/// Ids whose stored bounds intersect `region`, in id order.
	pub fn overlapping(&self, region: Aabb3d) -> Vec<Id> {
		let intersects =
			|id: &Id| self.entries.get(id).is_some_and(|e| e.bounds.intersects(&region));
		let mut ids: Vec<Id> = match &self.spatial {
			Some(spatial) if !self.region_is_sparse(region) => {
				let mut ids = spatial.query(region);
				ids.extend(self.unindexed.iter().copied().filter(intersects));
				ids
			}
			_ => self
				.entries
				.keys()
				.copied()
				.filter(|id| *id != Id::Universal)
				.filter(intersects)
				.collect(),
		};
		if self
			.entries
			.get(&Id::Universal)
			.is_some_and(|entry| entry.bounds.intersects(&region))
		{
			ids.push(Id::Universal);
		}
		ids.sort();
		ids
	}

	/// Whether walking the store beats enumerating `region`'s finest buckets.
	fn region_is_sparse(&self, region: Aabb3d) -> bool {
		let scale = self.base_scale;
		let extent = Vec3::from(region.max - region.min).as_dvec3().max(DVec3::ZERO);
		let buckets = (extent / scale).ceil().max(DVec3::ONE);
		buckets.x * buckets.y * buckets.z > 4.0 * self.entries.len().max(1) as f64
	}

	pub(super) fn put(&mut self, id: Id, entry: StoredEntry<T>, revision: u64) {
		self.index(id, entry.bounds);
		self.entries.insert(id, entry);
		self.membership_revision = revision;
	}

	pub(super) fn remove(&mut self, id: Id, revision: u64) -> Option<StoredEntry<T>> {
		let entry = self.entries.remove(&id)?;
		if let Some(spatial) = self.spatial.as_mut() {
			spatial.remove(id);
		}
		self.unindexed.remove(&id);
		self.membership_revision = revision;
		Some(entry)
	}

	/// Drops every entry, keeping the base scale.
	pub(super) fn reset(&mut self, revision: u64) {
		*self = Self::new(self.base_scale);
		self.membership_revision = revision;
	}
}
