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
use crate::shepherds::{ShepherdsCommuneDevelopment, ShepherdsVillageDevelopment};

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
	pub(crate) shepherds_communes: HashMap<Id, StoredEntry<ShepherdsCommuneDevelopment>>,
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

	pub fn shepherds_commune(&self, id: Id) -> Option<&ShepherdsCommuneDevelopment> {
		self.shepherds_communes.get(&id).map(|e| &e.value)
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
	($ty:ty, $field:ident, $view:ident) => {
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

		pub struct $view<'a> {
			store: &'a DevelopmentEntryStore,
		}

		impl<'a> $view<'a> {
			pub fn new(store: &'a DevelopmentEntryStore) -> Self {
				Self { store }
			}
		}

		impl SpatialIndex<$ty> for $view<'_> {
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
				self.store.$field.get(&id).map(|entry| &entry.value)
			}

			fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
				self.store.$field.get(&id).map(|entry| entry.bounds)
			}

			fn version(&self, id: Id) -> Option<Version> {
				self.store.$field.get(&id).map(|entry| entry.version)
			}

			fn insert(&mut self, _id: Id, _t: $ty, _bounds: Aabb3d, _lod_ref: &LodRef) {
				panic!(concat!(stringify!($view), " is read-only"));
			}
		}
	};
}

impl_spatial!(DevelopmentCell, cells, DevelopmentCellStoreView);
impl_spatial!(TerrainWithPads, padded, PaddedStoreView);
impl_spatial!(LesHallesDevelopment, les_halles, LesHallesStoreView);
impl_spatial!(ShepherdsVillageDevelopment, shepherds_villages, ShepherdsVillageStoreView);
impl_spatial!(ShepherdsCommuneDevelopment, shepherds_communes, ShepherdsCommuneStoreView);
