//! Spatial index of selected [`TerrainDetail`] cells and generated
//! [`TerrainOutcropping`]s. Generate and present read this; neither owns the other.

use std::collections::{HashMap, HashSet};

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use lod::gen::{Id, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;

use crate::{
	FormationExtent, FormationKind, OutcroppingExtent, TerrainDetail, TerrainOutcropping,
	DEFAULT_OUTCROPPING_EXTENT_XZ,
};

/// Storage for formation selections and 40 m outcropping origins.
#[derive(Resource, Clone)]
pub struct TerrainDetailIndex {
	next_version: u64,
	details: HashMap<Id, DetailEntry>,
	outcroppings: HashMap<Id, OutcroppingEntry>,
	outcropping_cells: HashMap<(i32, i32), Id>,
	pub noise: NoiseParams,
	/// Pinned formation for review cells. `None` uses the four-way throw.
	pub formation: Option<FormationKind>,
}

#[derive(Clone)]
struct DetailEntry {
	value: TerrainDetail,
	bounds: Aabb3d,
	version: Version,
}

#[derive(Clone)]
struct OutcroppingEntry {
	value: TerrainOutcropping,
	bounds: Aabb3d,
	version: Version,
}

impl Default for TerrainDetailIndex {
	fn default() -> Self {
		Self {
			next_version: 0,
			details: HashMap::new(),
			outcroppings: HashMap::new(),
			outcropping_cells: HashMap::new(),
			noise: NoiseParams::default(),
			formation: None,
		}
	}
}

impl TerrainDetailIndex {
	pub fn clear(&mut self) {
		self.details.clear();
		self.outcroppings.clear();
		self.outcropping_cells.clear();
		self.next_version = 0;
	}

	pub fn selected_formation_for(&self, extent: FormationExtent) -> FormationKind {
		match self.formation {
			Some(kind) => kind,
			None => FormationKind::throw_on(extent, self.noise),
		}
	}

	pub fn ensure_detail_selected(&mut self, extent: FormationExtent) {
		let id = extent.id();
		if self.details.contains_key(&id) {
			return;
		}
		let formation = self.selected_formation_for(extent);
		let version = self.next_version();
		self.details.insert(
			id,
			DetailEntry {
				value: TerrainDetail { extent, formation },
				bounds: extent.aabb(),
				version,
			},
		);
	}

	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	fn outcropping_grid_cell(bounds: Aabb3d) -> (i32, i32) {
		let s = DEFAULT_OUTCROPPING_EXTENT_XZ;
		((bounds.min.x / s).floor() as i32, (bounds.min.z / s).floor() as i32)
	}

	fn index_outcropping(&mut self, id: Id, bounds: Aabb3d) {
		self.outcropping_cells.insert(Self::outcropping_grid_cell(bounds), id);
	}

	fn unindex_outcropping(&mut self, bounds: Aabb3d) {
		self.outcropping_cells.remove(&Self::outcropping_grid_cell(bounds));
	}
}

impl SpatialIndex<TerrainDetail> for TerrainDetailIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.details
			.iter()
			.filter(|(_, entry)| region.intersects(&entry.bounds))
			.map(|(id, _)| TrackedId(*id))
			.collect()
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.details.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&TerrainDetail> {
		self.details.get(&id).map(|entry| &entry.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.details.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.details.get(&id).map(|entry| entry.version)
	}

	fn insert(&mut self, id: Id, t: TerrainDetail, bounds: Aabb3d, _lod_ref: &LodRef) {
		let version = self.next_version();
		self.details.insert(id, DetailEntry { value: t, bounds, version });
	}
}

impl SpatialIndex<TerrainOutcropping> for TerrainDetailIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		let mut seen = HashSet::new();
		let mut tracked = Vec::new();
		for cell in OutcroppingExtent::cells_overlapping(region) {
			let key = OutcroppingExtent::cell_index_containing(cell.center());
			let Some(&id) = self.outcropping_cells.get(&key) else {
				continue;
			};
			if !seen.insert(id) {
				continue;
			}
			let Some(entry) = self.outcroppings.get(&id) else {
				continue;
			};
			if region.intersects(&entry.bounds) {
				tracked.push(TrackedId(id));
			}
		}
		tracked
	}

	fn storage_status(&self, id: Id) -> StorageStatus {
		if self.outcroppings.contains_key(&id) {
			StorageStatus::TrackedWithin
		} else {
			StorageStatus::NotTracked
		}
	}

	fn get(&self, id: Id) -> Option<&TerrainOutcropping> {
		self.outcroppings.get(&id).map(|entry| &entry.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.outcroppings.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.outcroppings.get(&id).map(|entry| entry.version)
	}

	fn insert(&mut self, id: Id, t: TerrainOutcropping, bounds: Aabb3d, _lod_ref: &LodRef) {
		let version = self.next_version();
		if let Some(previous) = self.outcroppings.get(&id).map(|previous| previous.bounds) {
			self.unindex_outcropping(previous);
		}
		self.outcroppings.insert(id, OutcroppingEntry { value: t, bounds, version });
		self.index_outcropping(id, bounds);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::OutcroppingKind;
	use anyhow::Result;

	fn with_lod_ref<R>(f: impl FnOnce(&LodRef<'_>) -> R) -> R {
		let transform = Transform::IDENTITY;
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &transform,
			current_transform: &transform,
			bounds: &bounds,
		};
		f(&lod_ref)
	}

	#[test]
	fn outcropping_grid_returns_only_intersecting_cells() -> Result<()> {
		let near = OutcroppingExtent::from_cell_index(0, 0);
		let far = OutcroppingExtent::from_cell_index(8, 0);
		let mut index = TerrainDetailIndex::default();
		with_lod_ref(|lod_ref| {
			SpatialIndex::<TerrainOutcropping>::insert(
				&mut index,
				near.id(),
				TerrainOutcropping::selected(near, OutcroppingKind::Loner, NoiseParams::default()),
				near.aabb(),
				lod_ref,
			);
			SpatialIndex::<TerrainOutcropping>::insert(
				&mut index,
				far.id(),
				TerrainOutcropping::selected(far, OutcroppingKind::Loner, NoiseParams::default()),
				far.aabb(),
				lod_ref,
			);
		});
		let hits = SpatialIndex::<TerrainOutcropping>::tracked_ids_for(&index, near.aabb());
		assert_eq!(hits, vec![TrackedId(near.id())]);
		Ok(())
	}
}
