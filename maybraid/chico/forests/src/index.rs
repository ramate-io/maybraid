//! Spatial index of selected [`ChicoForest`] cells and generated [`ChicoGrove`]s.

use std::collections::{HashMap, HashSet};

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use chico_groves::FlatTerrainSample;
use lod::gen::{Id, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;

use crate::{
	select_cell, ChicoForest, ChicoGrove, ForestExtent, LayeringKind, NeighborLayers,
	SelectedLayers,
};

/// Storage for generated forest cells and grove tiles. Generation and
/// presentation read this; neither plugin owns the other.
#[derive(Resource, Clone)]
pub struct ForestIndex {
	next_version: u64,
	forests: HashMap<Id, ForestEntry>,
	groves: HashMap<Id, GroveEntry>,
	grove_cells: HashMap<(i32, i32), Vec<Id>>,
	pub noise: NoiseParams,
	pub layering: Option<LayeringKind>,
}

#[derive(Clone)]
struct ForestEntry {
	value: ChicoForest,
	bounds: Aabb3d,
	version: Version,
}

#[derive(Clone)]
struct GroveEntry {
	value: ChicoGrove,
	bounds: Aabb3d,
	version: Version,
}

impl Default for ForestIndex {
	fn default() -> Self {
		Self {
			next_version: 0,
			forests: HashMap::new(),
			groves: HashMap::new(),
			grove_cells: HashMap::new(),
			noise: NoiseParams::default(),
			layering: None,
		}
	}
}

impl ForestIndex {
	pub fn clear(&mut self) {
		self.forests.clear();
		self.groves.clear();
		self.grove_cells.clear();
		self.next_version = 0;
	}

	pub fn selected_layers_for(&self, extent: ForestExtent) -> SelectedLayers {
		match self.layering {
			Some(kind) => kind.layering().typical_layers(),
			None => select_cell(extent, self.noise),
		}
	}

	/// Insert the forest cell if missing (selection only). Used when listing grove origins.
	pub fn ensure_forest_selected(&mut self, extent: ForestExtent) {
		let id = extent.id();
		if self.forests.contains_key(&id) {
			return;
		}
		let layers = self.selected_layers_for(extent);
		let version = self.next_version();
		self.forests.insert(
			id,
			ForestEntry { value: ChicoForest { extent, layers }, bounds: extent.aabb(), version },
		);
	}

	pub fn neighbor_layers(&self, extent: ForestExtent) -> NeighborLayers {
		let (ix, iz) = ForestExtent::cell_index_containing(extent.center());
		NeighborLayers {
			north: self.layers_at(ix, iz + 1),
			east: self.layers_at(ix + 1, iz),
			south: self.layers_at(ix, iz - 1),
			west: self.layers_at(ix - 1, iz),
		}
	}

	fn layers_at(&self, ix: i32, iz: i32) -> Option<SelectedLayers> {
		let id = ForestExtent::from_cell_index(ix, iz).id();
		self.forests.get(&id).map(|entry| entry.value.layers)
	}

	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	fn index_grove(&mut self, id: Id, bounds: Aabb3d) {
		for cell in grove_cells_for(bounds) {
			self.grove_cells.entry(cell).or_default().push(id);
		}
	}

	fn unindex_grove(&mut self, id: Id, bounds: Aabb3d) {
		for cell in grove_cells_for(bounds) {
			let mut remove_cell = false;
			if let Some(ids) = self.grove_cells.get_mut(&cell) {
				ids.retain(|candidate| *candidate != id);
				remove_cell = ids.is_empty();
			}
			if remove_cell {
				self.grove_cells.remove(&cell);
			}
		}
	}
}

fn grove_cells_for(bounds: Aabb3d) -> impl Iterator<Item = (i32, i32)> {
	let cell = crate::DEFAULT_FOREST_GROVE_TILE_XZ;
	let min_x = (bounds.min.x / cell).floor() as i32;
	let min_z = (bounds.min.z / cell).floor() as i32;
	let max_x = (((bounds.max.x - 1e-3) / cell).floor() as i32).max(min_x);
	let max_z = (((bounds.max.z - 1e-3) / cell).floor() as i32).max(min_z);
	(min_x..=max_x).flat_map(move |x| (min_z..=max_z).map(move |z| (x, z)))
}

impl SpatialIndex<ChicoForest> for ForestIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.forests
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.forests.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&ChicoForest> {
		self.forests.get(&id).map(|entry| &entry.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.forests.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.forests.get(&id).map(|entry| entry.version)
	}

	fn insert(&mut self, id: Id, t: ChicoForest, bounds: Aabb3d, _lod_ref: &LodRef) {
		let version = self.next_version();
		self.forests.insert(id, ForestEntry { value: t, bounds, version });
	}

	fn storage_epoch(&self) -> u64 {
		self.next_version
	}
}

impl SpatialIndex<ChicoGrove> for ForestIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		let mut seen = HashSet::new();
		let mut tracked = Vec::new();
		for cell in grove_cells_for(region) {
			let Some(ids) = self.grove_cells.get(&cell) else {
				continue;
			};
			for &id in ids {
				if !seen.insert(id) {
					continue;
				}
				let Some(entry) = self.groves.get(&id) else {
					continue;
				};
				if region.min.x < entry.bounds.max.x
					&& region.max.x > entry.bounds.min.x
					&& region.min.z < entry.bounds.max.z
					&& region.max.z > entry.bounds.min.z
				{
					tracked.push(TrackedId(id));
				}
			}
		}
		tracked
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.groves.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&ChicoGrove> {
		self.groves.get(&id).map(|entry| &entry.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.groves.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.groves.get(&id).map(|entry| entry.version)
	}

	fn insert(&mut self, id: Id, t: ChicoGrove, bounds: Aabb3d, _lod_ref: &LodRef) {
		let version = self.next_version();
		if let Some(previous_bounds) = self.groves.get(&id).map(|previous| previous.bounds) {
			self.unindex_grove(id, previous_bounds);
		}
		self.groves.insert(id, GroveEntry { value: t, bounds, version });
		self.index_grove(id, bounds);
	}

	fn storage_epoch(&self) -> u64 {
		self.next_version
	}
}

/// World sample used when a presenter grows grove recipes.
pub fn forest_world_sample() -> FlatTerrainSample {
	FlatTerrainSample::default()
}
