//! Typed HCSG storage: one homogeneous [`NodeStore`] per generated type.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{DVec3, Vec3};
use bevy::prelude::Resource;

use crate::gen::{Id, SpatialIndex, StorageStatus, TrackedId, Version};

/// Base cell size of a store's spatial index when none is configured.
pub const DEFAULT_BASE_SCALE: DVec3 = DVec3::splat(64.0);

/// Any value HCSG can store.
pub trait HcsgNode: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> HcsgNode for T {}

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

	fn relocate(&mut self, id: Id, bounds: Aabb3d, revision: u64) -> bool {
		let Some(entry) = self.entries.get_mut(&id) else {
			return false;
		};
		entry.bounds = bounds;
		self.index(id, bounds);
		self.membership_revision = revision;
		true
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

trait ErasedStore: Send + Sync {
	fn as_any(&self) -> &dyn Any;
	fn as_any_mut(&mut self) -> &mut dyn Any;
	fn clear(&mut self, revision: u64);
}

impl<T: HcsgNode> ErasedStore for NodeStore<T> {
	fn as_any(&self) -> &dyn Any {
		self
	}

	fn as_any_mut(&mut self) -> &mut dyn Any {
		self
	}

	fn clear(&mut self, revision: u64) {
		self.reset(revision);
	}
}

/// Registry of [`NodeStore`]s keyed by Rust type.
///
/// [`TypeId`] picks the store and [`Id`] picks the instance inside it, so
/// two types can share an id (and `Id::Universal`) without colliding. Stores
/// are created on first insert; [`Self::configure`] sets a base scale and
/// group up front.
///
/// Versions come from one counter across all stores and never repeat, even
/// after a clear, so a presenter can never mistake a rebuild for the value it
/// already shows.
#[derive(Resource, Default)]
pub struct HcsgStorage {
	stores: HashMap<TypeId, Box<dyn ErasedStore>>,
	base_scales: HashMap<TypeId, DVec3>,
	groups: HashMap<TypeId, Vec<TypeId>>,
	next_version: u64,
}

impl HcsgStorage {
	/// Sets `T`'s spatial base scale. Tall cells want a tall Y so horizontal
	/// buckets stay fine. Rebuilds `T`'s index if it already exists.
	pub fn configure<T: HcsgNode>(&mut self, base_scale: DVec3) -> &mut Self {
		self.base_scales.insert(TypeId::of::<T>(), base_scale);
		if let Some(store) = self.store_mut_existing::<T>() {
			store.rebuild_index(base_scale);
		}
		self
	}

	/// Adds `T` to group `G`, so [`Self::clear_group`] drops it with the rest.
	pub fn add_to_group<G: 'static, T: HcsgNode>(&mut self) -> &mut Self {
		let members = self.groups.entry(TypeId::of::<G>()).or_default();
		if !members.contains(&TypeId::of::<T>()) {
			members.push(TypeId::of::<T>());
		}
		self
	}

	pub fn store<T: HcsgNode>(&self) -> Option<&NodeStore<T>> {
		self.stores.get(&TypeId::of::<T>())?.as_any().downcast_ref()
	}

	fn store_mut_existing<T: HcsgNode>(&mut self) -> Option<&mut NodeStore<T>> {
		self.stores.get_mut(&TypeId::of::<T>())?.as_any_mut().downcast_mut()
	}

	// Stores are keyed by their own `TypeId`, so the downcast cannot miss.
	#[allow(clippy::expect_used)]
	fn store_mut<T: HcsgNode>(&mut self) -> &mut NodeStore<T> {
		let base_scale =
			self.base_scales.get(&TypeId::of::<T>()).copied().unwrap_or(DEFAULT_BASE_SCALE);
		self.stores
			.entry(TypeId::of::<T>())
			.or_insert_with(|| Box::new(NodeStore::<T>::new(base_scale)))
			.as_any_mut()
			.downcast_mut()
			.expect("stores are keyed by their own TypeId")
	}

	fn stamp(&mut self) -> u64 {
		self.next_version += 1;
		self.next_version
	}

	pub fn get<T: HcsgNode>(&self, id: Id) -> Option<&T> {
		self.store::<T>()?.value(id)
	}

	pub fn entry<T: HcsgNode>(&self, id: Id) -> Option<&StoredEntry<T>> {
		self.store::<T>()?.entry(id)
	}

	pub fn contains<T: HcsgNode>(&self, id: Id) -> bool {
		self.store::<T>().is_some_and(|store| store.contains(id))
	}

	/// Stored ids of `T` whose bounds intersect `region`.
	pub fn overlapping<T: HcsgNode>(&self, region: Aabb3d) -> Vec<Id> {
		self.store::<T>().map(|store| store.overlapping(region)).unwrap_or_default()
	}

	/// Stores `value` under `id`, stamping a fresh version.
	pub fn insert<T: HcsgNode>(&mut self, id: Id, value: T, bounds: Aabb3d) -> Version {
		let stamp = self.stamp();
		let version = Version(stamp);
		self.store_mut::<T>().put(id, StoredEntry { value, bounds, version }, stamp);
		version
	}

	/// Stores a root input at `Id::Universal`.
	///
	/// Root inputs are the few values nothing can generate (a seed, mode
	/// configuration). Everything derived from them is a [`crate::gen::GenerationScheme`].
	pub fn seed<T: HcsgNode>(&mut self, value: T, bounds: Aabb3d) -> Version {
		self.insert(Id::Universal, value, bounds)
	}

	/// Moves a tracked entry. The version is unchanged: the value did not change.
	pub fn relocate<T: HcsgNode>(&mut self, id: Id, bounds: Aabb3d) -> bool {
		let stamp = self.stamp();
		self.store_mut_existing::<T>()
			.is_some_and(|store| store.relocate(id, bounds, stamp))
	}

	pub fn remove<T: HcsgNode>(&mut self, id: Id) -> Option<T> {
		let stamp = self.stamp();
		Some(self.store_mut_existing::<T>()?.remove(id, stamp)?.value)
	}

	pub fn clear<T: HcsgNode>(&mut self) {
		let stamp = self.stamp();
		if let Some(store) = self.stores.get_mut(&TypeId::of::<T>()) {
			store.clear(stamp);
		}
	}

	/// Drops every store in group `G`.
	pub fn clear_group<G: 'static>(&mut self) {
		let stamp = self.stamp();
		let Some(members) = self.groups.get(&TypeId::of::<G>()) else {
			return;
		};
		for member in members {
			if let Some(store) = self.stores.get_mut(member) {
				store.clear(stamp);
			}
		}
	}

	/// Last version handed out by any store.
	pub fn latest_version(&self) -> u64 {
		self.next_version
	}
}

impl<T: HcsgNode> SpatialIndex<T> for HcsgStorage {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.overlapping::<T>(region).into_iter().map(TrackedId).collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		let Some(entry) = self.entry::<T>(id) else {
			return StorageStatus::NotTracked;
		};
		match id.origin_cell_bounds() {
			Some(origin) if !origin.intersects(&entry.bounds) => StorageStatus::TrackedOutside,
			_ => StorageStatus::TrackedWithin,
		}
	}

	fn get(&self, id: Id) -> Option<&T> {
		HcsgStorage::get(self, id)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.entry::<T>(id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.entry::<T>(id).map(|entry| entry.version)
	}

	fn membership_revision(&self) -> u64 {
		self.store::<T>().map_or(0, NodeStore::membership_revision)
	}

	fn insert(&mut self, id: Id, value: T, bounds: Aabb3d) {
		HcsgStorage::insert(self, id, value, bounds);
	}
}
