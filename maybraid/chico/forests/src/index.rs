//! [`SpatialIndex`] of assembled [`ChicoForest`] cells.

use std::collections::HashMap;

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use chico_groves::FlatTerrainSample;
use lod::gen::{Id, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;

use crate::{ChicoForest, ForestExtent, LayeringKind, NeighborLayers, SelectedLayers};

/// Storage for generated forest cells. Generation and presentation read this;
/// neither plugin owns the other.
#[derive(Resource, Clone)]
pub struct ForestIndex {
	next_version: u64,
	entries: HashMap<Id, ForestEntry>,
	pub noise: NoiseParams,
	pub layering: Option<LayeringKind>,
}

#[derive(Clone)]
struct ForestEntry {
	value: ChicoForest,
	bounds: Aabb3d,
	version: Version,
}

impl Default for ForestIndex {
	fn default() -> Self {
		Self {
			next_version: 0,
			entries: HashMap::new(),
			noise: NoiseParams::default(),
			layering: None,
		}
	}
}

impl ForestIndex {
	pub fn clear(&mut self) {
		self.entries.clear();
		self.next_version = 0;
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
		self.entries.get(&id).map(|entry| entry.value.assembled.layers)
	}

	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}
}

impl SpatialIndex<ChicoForest> for ForestIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.entries
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.entries.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&ChicoForest> {
		self.entries.get(&id).map(|entry| &entry.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.entries.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.entries.get(&id).map(|entry| entry.version)
	}

	fn insert(&mut self, id: Id, t: ChicoForest, bounds: Aabb3d, _lod_ref: &LodRef) {
		let version = self.next_version();
		self.entries.insert(id, ForestEntry { value: t, bounds, version });
	}
}

/// World sample used when assembling a forest cell into the index.
pub fn forest_world_sample() -> FlatTerrainSample {
	FlatTerrainSample::default()
}
