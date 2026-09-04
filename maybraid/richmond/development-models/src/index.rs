//! Spatial store for development cells, padded terrain, and Les Halles hosts.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use lod::gen::{Id, OriginalId, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use procedural_common::Bounds2;
use std::collections::{HashMap, HashSet};

use richmond_urbanization::UrbanizationIndex;

use crate::artifact::BuiltDevelopment;
use crate::config::DevelopmentConfig;
use crate::development::DevelopmentCell;
use crate::pad::PadComplex;
use crate::padded::TerrainWithPads;

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
	pad_cells: HashMap<(i32, i32), Vec<Id>>,
	pad_cells_by_development: HashMap<Id, Vec<(i32, i32)>>,
	merged_pads: HashMap<Id, PadComplex>,
	pub(crate) padded: HashMap<Id, StoredEntry<TerrainWithPads>>,
	pub(crate) developments: HashMap<Id, StoredEntry<BuiltDevelopment>>,
	dirty_pad_regions: Vec<Bounds2>,
}

impl DevelopmentEntryStore {
	pub fn clear(&mut self) {
		self.cells.clear();
		self.pad_cells.clear();
		self.pad_cells_by_development.clear();
		self.merged_pads.clear();
		self.padded.clear();
		self.developments.clear();
		self.dirty_pad_regions.clear();
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

	/// Merge pad nodes affecting `region` into one sample-time blend pass.
	///
	/// Returning one complex is important for overlapping pads: sequential
	/// modulation would let later ease skirts smear earlier exact terraces.
	pub fn merged_pad_complex(&mut self, region: Aabb3d) -> PadComplex {
		let cache_key = Id::from_cell(region);
		if let Some(cached) = self.merged_pads.get(&cache_key) {
			return cached.clone();
		}
		let bounds = Bounds2::from_xz(region.min.x, region.min.z, region.max.x, region.max.z);
		let mut ids = HashSet::new();
		for cell in pad_index_cells(bounds) {
			if let Some(indexed) = self.pad_cells.get(&cell) {
				ids.extend(indexed.iter().copied());
			}
		}
		let nodes = ids
			.into_iter()
			.filter_map(|id| self.cells.get(&id))
			.flat_map(|entry| entry.value.pad_complexes())
			.flat_map(|complex| complex.pads.iter())
			.filter(|node| node.correction_intersects(bounds))
			.cloned()
			.collect();
		let merged = PadComplex::from_nodes(nodes);
		self.merged_pads.insert(cache_key, merged.clone());
		merged
	}

	fn mark_development_change(&mut self, id: Id, development: &DevelopmentCell) {
		self.unindex_development_pads(id);
		self.merged_pads.clear();
		let previous: Vec<_> = self
			.cells
			.get(&id)
			.into_iter()
			.flat_map(|entry| entry.value.pad_complexes())
			.filter(|complex| !complex.is_empty())
			.map(|complex| complex.bounds)
			.collect();
		self.dirty_pad_regions.extend(previous);
		self.dirty_pad_regions.extend(
			development
				.pad_complexes()
				.filter(|complex| !complex.is_empty())
				.map(|complex| complex.bounds),
		);
		self.index_development_pads(id, development);
	}

	fn index_development_pads(&mut self, id: Id, development: &DevelopmentCell) {
		let mut indexed_cells = HashSet::new();
		for complex in development.pad_complexes().filter(|complex| !complex.is_empty()) {
			for cell in pad_index_cells(complex.bounds) {
				if !indexed_cells.insert(cell) {
					continue;
				}
				let ids = self.pad_cells.entry(cell).or_default();
				if !ids.contains(&id) {
					ids.push(id);
				}
			}
		}
		if !indexed_cells.is_empty() {
			self.pad_cells_by_development.insert(id, indexed_cells.into_iter().collect());
		}
	}

	fn unindex_development_pads(&mut self, id: Id) {
		let Some(indexed_cells) = self.pad_cells_by_development.remove(&id) else {
			return;
		};
		for cell in indexed_cells {
			let remove_cell = self.pad_cells.get_mut(&cell).is_some_and(|ids| {
				ids.retain(|candidate| *candidate != id);
				ids.is_empty()
			});
			if remove_cell {
				self.pad_cells.remove(&cell);
			}
		}
	}

	/// Remove only padded terrain cells touched by changed pad support.
	pub fn invalidate_dirty_padded(&mut self) -> usize {
		let dirty = std::mem::take(&mut self.dirty_pad_regions);
		if dirty.is_empty() {
			return 0;
		}
		let previous_len = self.padded.len();
		self.padded.retain(|_, entry| {
			!dirty.iter().any(|region| {
				region.min.x < entry.bounds.max.x
					&& region.max.x > entry.bounds.min.x
					&& region.min.y < entry.bounds.max.z
					&& region.max.y > entry.bounds.min.z
			})
		});
		previous_len - self.padded.len()
	}

	pub fn padded(&self, id: Id) -> Option<&TerrainWithPads> {
		self.padded.get(&id).map(|e| &e.value)
	}

	/// Finest padded terrain cell with the greatest XZ overlap with `region`.
	pub fn padded_terrain_for(&self, region: Aabb3d) -> Option<&TerrainWithPads> {
		let mut best: Option<(f32, f32, &TerrainWithPads)> = None;
		for entry in self.padded.values() {
			let overlap_x = (region.max.x.min(entry.bounds.max.x)
				- region.min.x.max(entry.bounds.min.x))
			.max(0.0);
			let overlap_z = (region.max.z.min(entry.bounds.max.z)
				- region.min.z.max(entry.bounds.min.z))
			.max(0.0);
			let overlap = overlap_x * overlap_z;
			if overlap <= 1e-3 {
				continue;
			}
			let span = (entry.bounds.max.x - entry.bounds.min.x)
				.max(entry.bounds.max.z - entry.bounds.min.z);
			if best.is_none_or(|(best_overlap, best_span, _)| {
				overlap > best_overlap || (overlap == best_overlap && span < best_span)
			}) {
				best = Some((overlap, span, &entry.value));
			}
		}
		best.map(|(_, _, terrain)| terrain)
	}

	pub fn development(&self, id: Id) -> Option<&BuiltDevelopment> {
		self.developments.get(&id).map(|e| &e.value)
	}
}

const PAD_INDEX_CELL_XZ: f32 = 160.0;

fn pad_index_cells(bounds: Bounds2) -> impl Iterator<Item = (i32, i32)> {
	let min_x = (bounds.min.x / PAD_INDEX_CELL_XZ).floor() as i32;
	let min_z = (bounds.min.y / PAD_INDEX_CELL_XZ).floor() as i32;
	let max_x = (((bounds.max.x - 1e-3) / PAD_INDEX_CELL_XZ).floor() as i32).max(min_x);
	let max_z = (((bounds.max.y - 1e-3) / PAD_INDEX_CELL_XZ).floor() as i32).max(min_z);
	(min_x..=max_x).flat_map(move |x| (min_z..=max_z).map(move |z| (x, z)))
}

/// System-local index: development store plus read-only Durham terrain.
#[derive(SystemParam)]
pub struct DevelopmentIndex<'w> {
	pub store: ResMut<'w, DevelopmentEntryStore>,
	pub terrain: Res<'w, TerrainEntryStore>,
	pub layout: Res<'w, TerrainCellLayout>,
	pub config: Res<'w, DevelopmentConfig>,
	pub urbanization: ResMut<'w, UrbanizationIndex>,
}

impl DevelopmentIndex<'_> {
	pub fn clear(&mut self) {
		self.store.clear();
		self.urbanization.clear();
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
	($ty:ty, $field:ident, $view:ident $(, $before_insert:ident)?) => {
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
				$(self.store.$before_insert(id, &t);)?
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

impl_spatial!(DevelopmentCell, cells, DevelopmentCellStoreView, mark_development_change);
impl_spatial!(TerrainWithPads, padded, PaddedStoreView);
impl_spatial!(BuiltDevelopment, developments, BuiltDevelopmentStoreView);

#[cfg(test)]
mod tests {
	use super::*;
	use crate::DevelopmentConfig;
	use anyhow::Result;
	use durham_terrain_models::{ComposedTerrain, TerrainSdf};
	use render_item::sdf::cpu_shot::WallFaces;
	use std::sync::Arc;

	#[test]
	fn merged_pad_complex_collects_overlapping_cells_in_one_pass() -> Result<()> {
		let mut store = DevelopmentEntryStore::default();
		let config = DevelopmentConfig::default();
		let first_bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(100.0, 100.0, 100.0));
		let second_bounds =
			Aabb3d::from_min_max(Vec3::new(100.0, 0.0, 0.0), Vec3::new(200.0, 100.0, 100.0));
		let first_id = Id::from_cell(first_bounds);
		let first = DevelopmentCell::with_les_halles(first_bounds, 12.0, &config);
		store.mark_development_change(first_id, &first);
		store.cells.insert(
			first_id,
			StoredEntry { value: first, bounds: first_bounds, version: Version(1) },
		);
		let second_id = Id::from_cell(second_bounds);
		let second = DevelopmentCell::with_les_halles(second_bounds, 24.0, &config);
		store.mark_development_change(second_id, &second);
		store.cells.insert(
			second_id,
			StoredEntry { value: second, bounds: second_bounds, version: Version(2) },
		);

		let merged = store.merged_pad_complex(Aabb3d::from_min_max(
			Vec3::new(40.0, 500.0, 0.0),
			Vec3::new(160.0, 501.0, 100.0),
		));
		assert_eq!(merged.pads.len(), 2);
		assert!((merged.modify_elevation(0.0, 50.0, 50.0) - 12.0).abs() < 1e-5);
		assert!((merged.modify_elevation(7.0, 500.0, 500.0) - 7.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn development_changes_only_invalidate_overlapping_padded_terrain() {
		let mut store = DevelopmentEntryStore::default();
		let config = DevelopmentConfig::default();
		let changed_bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(100.0, 100.0, 100.0));
		let distant_bounds = Aabb3d::from_min_max(
			Vec3::new(1_000.0, 0.0, 1_000.0),
			Vec3::new(1_100.0, 100.0, 1_100.0),
		);
		let development = DevelopmentCell::with_les_halles(changed_bounds, 12.0, &config);
		store.mark_development_change(Id::from_cell(changed_bounds), &development);

		for bounds in [changed_bounds, distant_bounds] {
			store.padded.insert(
				Id::from_cell(bounds),
				StoredEntry {
					value: TerrainWithPads {
						cell: bounds,
						sdf: Arc::new(ComposedTerrain::from_terrain(TerrainSdf::new(1, 20.0))),
						material: Handle::default(),
						res_2: 3,
						wall_faces: WallFaces::NONE,
						pad_count: 0,
					},
					bounds,
					version: Version(1),
				},
			);
		}

		assert_eq!(store.invalidate_dirty_padded(), 1);
		assert!(!store.padded.contains_key(&Id::from_cell(changed_bounds)));
		assert!(store.padded.contains_key(&Id::from_cell(distant_bounds)));
	}
}
