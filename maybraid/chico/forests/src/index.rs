//! Spatial index of selected [`ChicoForest`] cells and generated [`ChicoGrove`]s.

use std::collections::HashMap;

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
			noise: NoiseParams::default(),
			layering: None,
		}
	}
}

impl ForestIndex {
	pub fn clear(&mut self) {
		self.forests.clear();
		self.groves.clear();
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
		self.groves
			.iter()
			.filter(|(_, entry)| {
				region.min.x < entry.bounds.max.x
					&& region.max.x > entry.bounds.min.x
					&& region.min.z < entry.bounds.max.z
					&& region.max.z > entry.bounds.min.z
			})
			.map(|(id, _)| TrackedId(*id))
			.collect()
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
		self.groves.insert(id, GroveEntry { value: t, bounds, version });
	}

	fn storage_epoch(&self) -> u64 {
		self.next_version
	}
}

/// World sample used when a presenter grows grove recipes.
pub fn forest_world_sample() -> FlatTerrainSample {
	FlatTerrainSample::default()
}
