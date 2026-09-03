//! Spatial store for development cells, padded terrain, and Les Halles hosts.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use lod::gen::{Id, OriginalId, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use std::collections::HashMap;

use crate::config::DevelopmentConfig;
use crate::development::DevelopmentCell;
use crate::les_halles::LesHallesDevelopment;
use crate::padded::TerrainWithPads;
use crate::shepherds::ShepherdsVillageDevelopment;

#[derive(Debug, Clone)]
pub(crate) struct StoredEntry<T> {
	pub(crate) value: T,
	pub(crate) bounds: Aabb3d,
	pub(crate) version: Version,
}

/// Side table for Richmond development generation layers.
#[derive(Resource, Default)]
pub struct DevelopmentEntryStore {
	next_version: u64,
	pub(crate) cells: HashMap<Id, StoredEntry<DevelopmentCell>>,
	pub(crate) padded: HashMap<Id, StoredEntry<TerrainWithPads>>,
	pub(crate) les_halles: HashMap<Id, StoredEntry<LesHallesDevelopment>>,
	pub(crate) shepherds_villages: HashMap<Id, StoredEntry<ShepherdsVillageDevelopment>>,
}

impl DevelopmentEntryStore {
	pub fn clear(&mut self) {
		*self = Self::default();
	}

	fn stamp(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	pub fn cell(&self, id: Id) -> Option<&DevelopmentCell> {
		self.cells.get(&id).map(|e| &e.value)
	}

	pub fn filled_cells_overlapping(&self, region: Aabb3d) -> Vec<&DevelopmentCell> {
		self.cells
			.values()
			.filter(|e| e.value.is_filled() && region.intersects(&e.bounds))
			.map(|e| &e.value)
			.collect()
	}

	pub fn filled_original_ids(&self, region: Aabb3d) -> Vec<OriginalId> {
		self.cells
			.iter()
			.filter(|(_, e)| e.value.is_filled() && region.intersects(&e.bounds))
			.map(|(id, _)| OriginalId(*id))
			.collect()
	}

	pub fn les_halles(&self, id: Id) -> Option<&LesHallesDevelopment> {
		self.les_halles.get(&id).map(|e| &e.value)
	}

	pub fn padded(&self, id: Id) -> Option<&TerrainWithPads> {
		self.padded.get(&id).map(|e| &e.value)
	}

	pub fn shepherds_village(&self, id: Id) -> Option<&ShepherdsVillageDevelopment> {
		self.shepherds_villages.get(&id).map(|e| &e.value)
	}
}

/// System-local index: development store plus read-only Durham terrain.
#[derive(SystemParam)]
pub struct DevelopmentIndex<'w> {
	pub store: ResMut<'w, DevelopmentEntryStore>,
	pub terrain: Res<'w, TerrainEntryStore>,
	pub layout: Res<'w, TerrainCellLayout>,
	pub config: Res<'w, DevelopmentConfig>,
}

impl DevelopmentIndex<'_> {
	pub fn clear(&mut self) {
		self.store.clear();
	}

	pub fn config(&self) -> &DevelopmentConfig {
		&self.config
	}

	pub fn layout(&self) -> &TerrainCellLayout {
		&self.layout
	}

	pub fn terrain_store(&self) -> &TerrainEntryStore {
		&self.terrain
	}
}

macro_rules! impl_spatial {
	($ty:ty, $field:ident) => {
		impl<'w> SpatialIndex<$ty> for DevelopmentIndex<'w> {
			fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
				self.store
					.$field
					.iter()
					.filter(|(_, entry)| region.intersects(&entry.bounds))
					.map(|(id, _)| TrackedId(*id))
					.collect()
			}

			fn storage_status(&self, id: Id) -> StorageStatus {
				if self.store.$field.contains_key(&id) {
					StorageStatus::TrackedWithin
				} else {
					StorageStatus::NotTracked
				}
			}

			fn get(&self, id: Id) -> Option<&$ty> {
				self.store.$field.get(&id).map(|e| &e.value)
			}

			fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
				self.store.$field.get(&id).map(|e| e.bounds)
			}

			fn version(&self, id: Id) -> Option<Version> {
				self.store.$field.get(&id).map(|e| e.version)
			}

			fn insert(&mut self, id: Id, t: $ty, bounds: Aabb3d, _lod_ref: &LodRef) {
				let version = self.store.stamp();
				self.store.$field.insert(id, StoredEntry { value: t, bounds, version });
			}
		}
	};
}

impl_spatial!(DevelopmentCell, cells);
impl_spatial!(TerrainWithPads, padded);
impl_spatial!(LesHallesDevelopment, les_halles);
impl_spatial!(ShepherdsVillageDevelopment, shepherds_villages);

/// Read-only view over padded terrain for presentation.
pub struct PaddedStoreView<'a> {
	store: &'a DevelopmentEntryStore,
}

impl<'a> PaddedStoreView<'a> {
	pub fn new(store: &'a DevelopmentEntryStore) -> Self {
		Self { store }
	}
}

impl SpatialIndex<TerrainWithPads> for PaddedStoreView<'_> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.store
			.padded
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.store.padded.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&TerrainWithPads> {
		self.store.padded.get(&id).map(|e| &e.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.store.padded.get(&id).map(|e| e.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.store.padded.get(&id).map(|e| e.version)
	}

	fn insert(&mut self, _id: Id, _t: TerrainWithPads, _bounds: Aabb3d, _lod_ref: &LodRef) {
		panic!("PaddedStoreView is read-only");
	}
}

/// Read-only view over fitted Les Halles developments.
pub struct LesHallesStoreView<'a> {
	store: &'a DevelopmentEntryStore,
}

impl<'a> LesHallesStoreView<'a> {
	pub fn new(store: &'a DevelopmentEntryStore) -> Self {
		Self { store }
	}
}

impl SpatialIndex<LesHallesDevelopment> for LesHallesStoreView<'_> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.store
			.les_halles
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.store.les_halles.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&LesHallesDevelopment> {
		self.store.les_halles.get(&id).map(|e| &e.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.store.les_halles.get(&id).map(|e| e.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.store.les_halles.get(&id).map(|e| e.version)
	}

	fn insert(&mut self, _id: Id, _t: LesHallesDevelopment, _bounds: Aabb3d, _lod_ref: &LodRef) {
		panic!("LesHallesStoreView is read-only");
	}
}

/// Read-only view over fitted Shepherds Village developments.
pub struct ShepherdsVillageStoreView<'a> {
	store: &'a DevelopmentEntryStore,
}

impl<'a> ShepherdsVillageStoreView<'a> {
	pub fn new(store: &'a DevelopmentEntryStore) -> Self {
		Self { store }
	}
}

impl SpatialIndex<ShepherdsVillageDevelopment> for ShepherdsVillageStoreView<'_> {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.store
			.shepherds_villages
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.store.shepherds_villages.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&ShepherdsVillageDevelopment> {
		self.store.shepherds_villages.get(&id).map(|e| &e.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.store.shepherds_villages.get(&id).map(|e| e.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.store.shepherds_villages.get(&id).map(|e| e.version)
	}

	fn insert(
		&mut self,
		_id: Id,
		_t: ShepherdsVillageDevelopment,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
		panic!("ShepherdsVillageStoreView is read-only");
	}
}
