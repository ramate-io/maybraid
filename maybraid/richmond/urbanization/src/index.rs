//! Spatial index of selected [`SelectedUrbanization`] cells.

use std::collections::HashMap;

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use lod::gen::{Id, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;

use crate::{
	select_cell, select_cell_as, DevelopmentLeaf, SelectedUrbanization, UrbanizationExtent,
	UrbanizationKind,
};

/// Storage for selected urbanization cells.
#[derive(Resource, Clone)]
pub struct UrbanizationIndex {
	next_version: u64,
	cells: HashMap<Id, Entry>,
	pub noise: NoiseParams,
	/// When set, every cell uses this kind instead of Hopscotch (playground pin).
	pub kind: Option<UrbanizationKind>,
}

#[derive(Clone)]
struct Entry {
	value: SelectedUrbanization,
	bounds: Aabb3d,
	version: Version,
}

impl Default for UrbanizationIndex {
	fn default() -> Self {
		Self { next_version: 0, cells: HashMap::new(), noise: NoiseParams::default(), kind: None }
	}
}

impl UrbanizationIndex {
	pub fn clear(&mut self) {
		self.cells.clear();
		self.next_version = 0;
	}

	/// Insert the urbanization cell if missing (selection only).
	pub fn ensure_selected(&mut self, extent: UrbanizationExtent, noise: NoiseParams) {
		let id = extent.id();
		if self.cells.contains_key(&id) {
			return;
		}
		let value = match self.kind {
			Some(kind) => select_cell_as(extent, noise, kind),
			None => select_cell(extent, noise),
		};
		let version = self.next_version();
		self.cells.insert(id, Entry { value, bounds: extent.aabb(), version });
	}

	pub fn get(&self, id: Id) -> Option<&SelectedUrbanization> {
		self.cells.get(&id).map(|entry| &entry.value)
	}

	/// Look up a guillotine leaf by its [`DevelopmentLeaf::id`].
	pub fn leaf(&self, id: Id) -> Option<&DevelopmentLeaf> {
		self.cells
			.values()
			.find_map(|entry| entry.value.leaves.iter().find(|leaf| leaf.id() == id))
	}

	/// Alias for [`Self::leaf`].
	pub fn find_leaf(&self, id: Id) -> Option<&DevelopmentLeaf> {
		self.leaf(id)
	}

	/// Non-empty leaves whose bounds intersect `region`.
	pub fn filled_leaves_overlapping(&self, region: Aabb3d) -> Vec<&DevelopmentLeaf> {
		self.cells
			.values()
			.flat_map(|entry| entry.value.leaves.iter())
			.filter(|leaf| {
				leaf.kind != crate::UrbanDevelopmentKind::Empty && region.intersects(&leaf.bounds)
			})
			.collect()
	}

	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}
}

impl SpatialIndex<SelectedUrbanization> for UrbanizationIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.cells
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.cells.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&SelectedUrbanization> {
		UrbanizationIndex::get(self, id)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.cells.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.cells.get(&id).map(|entry| entry.version)
	}

	fn insert(&mut self, id: Id, t: SelectedUrbanization, bounds: Aabb3d, _lod_ref: &LodRef) {
		let version = self.next_version();
		self.cells.insert(id, Entry { value: t, bounds, version });
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn ensure_selected_is_idempotent() -> Result<()> {
		let mut index = UrbanizationIndex::default();
		let extent = UrbanizationExtent::default_cell();
		let noise = NoiseParams::from_scalar(1.0, 0.01, 1.0, 1);
		index.ensure_selected(extent, noise);
		index.ensure_selected(extent, noise);
		assert_eq!(index.cells.len(), 1);
		assert!(index.get(extent.id()).is_some());
		Ok(())
	}

	#[test]
	fn leaf_lookup_finds_non_empty_leaf() -> Result<()> {
		let mut index = UrbanizationIndex {
			kind: Some(UrbanizationKind::Frontier),
			..UrbanizationIndex::default()
		};
		let extent = UrbanizationExtent::default_cell();
		let noise = NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1);
		index.ensure_selected(extent, noise);
		let selected = index.get(extent.id()).ok_or_else(|| anyhow::anyhow!("cell"))?;
		let Some(leaf) = selected.leaves.first() else {
			return Err(anyhow::anyhow!("expected leaves"));
		};
		assert_eq!(index.leaf(leaf.id()).map(|l| l.id()), Some(leaf.id()));
		Ok(())
	}
}
