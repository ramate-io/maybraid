//! Generated 50 m furniture cells.

use std::collections::{HashMap, HashSet};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use building_components::{FurnitureNode, Placement};
use lod::gen::{GenerationScheme, Id, OriginalId, SpatialIndex, StorageStatus, TrackedId, Version};

use crate::cell::{intersects_xz, FurnitureCellExtent};
use crate::host::FurnitureCell;

#[derive(Clone)]
struct StoredFurnitureCell {
	value: FurnitureCell,
	bounds: Aabb3d,
	version: Version,
}

struct CachedDevelopmentSlots {
	version: Version,
	slots: Vec<FurnitureNode>,
}

/// Generated 50 m furniture cells plus the current world-space slot list.
#[derive(Resource, Default)]
pub struct FurnitureIndex {
	next_version: u64,
	cells: HashMap<Id, StoredFurnitureCell>,
	slots: Vec<FurnitureNode>,
	development_slots: HashMap<Id, CachedDevelopmentSlots>,
	slots_fingerprint: Vec<(Id, Version)>,
	slots_region: Option<Aabb3d>,
}

impl FurnitureIndex {
	fn next_version(&mut self) -> Version {
		self.next_version += 1;
		Version(self.next_version)
	}

	/// Generated 50 m host count (HUD).
	pub fn cell_count(&self) -> usize {
		self.cells.len()
	}

	/// Flattened High slots currently in the generate ring (HUD).
	pub fn slot_count(&self) -> usize {
		self.slots.len()
	}

	pub(crate) fn clear(&mut self) {
		self.cells.clear();
		self.slots.clear();
		self.development_slots.clear();
		self.slots_fingerprint.clear();
		self.slots_region = None;
	}

	pub(crate) fn remove(&mut self, id: Id) -> bool {
		if self.cells.remove(&id).is_none() {
			return false;
		}
		let _ = self.next_version();
		true
	}

	/// Refresh flattened slots. `expand` runs only for developments whose
	/// version is not already cached, and only after the region/fingerprint miss.
	pub(crate) fn refresh_slots(
		&mut self,
		tracked: Vec<(Id, Version)>,
		region: Aabb3d,
		mut expand: impl FnMut(Id) -> Vec<FurnitureNode>,
	) {
		let mut fingerprint = tracked.clone();
		fingerprint.sort();
		if fingerprint == self.slots_fingerprint
			&& self.slots_region.is_some_and(|cached| regions_match(cached, region))
		{
			return;
		}
		let live: HashSet<Id> = tracked.iter().map(|(id, _)| *id).collect();
		self.development_slots.retain(|id, _| live.contains(id));
		self.slots.clear();
		for (id, version) in tracked {
			let cached = self.development_slots.entry(id).or_insert_with(|| {
				CachedDevelopmentSlots { version: Version(0), slots: Vec::new() }
			});
			if cached.version != version {
				cached.slots = expand(id);
				cached.version = version;
			}
			self.slots
				.extend(cached.slots.iter().filter(|slot| slot_in_region(slot, region)).cloned());
		}
		self.slots_fingerprint = fingerprint;
		self.slots_region = Some(region);
	}

	pub(crate) fn generate_cells(&mut self, region: Aabb3d, budget: usize) -> usize {
		let mut built = 0usize;
		for OriginalId(id) in FurnitureCell::original_ids_for(self, region) {
			match FurnitureCell::build_with_id(self, id) {
				Some((cell, bounds)) => {
					if self.get(id).is_some_and(|existing| same_slots(existing, &cell)) {
						continue;
					}
					if built >= budget {
						break;
					}
					self.insert(id, cell, bounds);
					built += 1;
				}
				None => {
					if !self.cells.contains_key(&id) {
						continue;
					}
					if built >= budget {
						break;
					}
					self.remove(id);
					built += 1;
				}
			}
		}
		built
	}
}

fn regions_match(left: Aabb3d, right: Aabb3d) -> bool {
	(left.min - right.min).length_squared() < 1e-6 && (left.max - right.max).length_squared() < 1e-6
}

fn slot_in_region(slot: &FurnitureNode, region: Aabb3d) -> bool {
	let p = slot.placement.translation;
	p.x >= region.min.x && p.x <= region.max.x && p.z >= region.min.z && p.z <= region.max.z
}

fn cell_id_for_slot(slot: &FurnitureNode) -> Id {
	let (ix, iz) = FurnitureCellExtent::cell_index_containing(slot.placement.translation);
	FurnitureCellExtent::from_cell_index(ix, iz).id()
}

fn placement_match(left: &Placement, right: &Placement) -> bool {
	(left.translation - right.translation).length_squared() < 1e-4
		&& (left.yaw - right.yaw).abs() < 1e-4
		&& (left.pitch - right.pitch).abs() < 1e-4
		&& (left.roll - right.roll).abs() < 1e-4
		&& (left.scale - right.scale).length_squared() < 1e-4
}

fn slots_match(left: &[FurnitureNode], right: &[FurnitureNode]) -> bool {
	left.len() == right.len()
		&& left
			.iter()
			.zip(right)
			.all(|(a, b)| a.geometry == b.geometry && placement_match(&a.placement, &b.placement))
}

impl SpatialIndex<FurnitureCell> for FurnitureIndex {
	fn tracked_ids_for(&self, region: Aabb3d) -> Vec<TrackedId> {
		self.cells
			.iter()
			.filter(|(_, entry)| intersects_xz(region, entry.bounds))
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

	fn get(&self, id: Id) -> Option<&FurnitureCell> {
		self.cells.get(&id).map(|entry| &entry.value)
	}

	fn get_bounds(&self, id: Id) -> Option<Aabb3d> {
		self.cells.get(&id).map(|entry| entry.bounds)
	}

	fn version(&self, id: Id) -> Option<Version> {
		self.cells.get(&id).map(|entry| entry.version)
	}

	fn membership_revision(&self) -> u64 {
		self.next_version
	}

	fn insert(&mut self, id: Id, value: FurnitureCell, bounds: Aabb3d) {
		let version = self.next_version();
		self.cells.insert(id, StoredFurnitureCell { value, bounds, version });
	}
}

impl GenerationScheme<FurnitureIndex> for FurnitureCell {
	fn original_ids_for(index: &mut FurnitureIndex, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids = HashSet::new();
		for slot in &index.slots {
			ids.insert(cell_id_for_slot(slot));
		}
		for id in index.cells.keys().copied() {
			let Some(extent) = FurnitureCellExtent::from_id(id) else {
				continue;
			};
			if intersects_xz(region, extent.aabb()) {
				ids.insert(id);
			}
		}
		ids.into_iter().map(OriginalId).collect()
	}

	fn build_with_id(index: &mut FurnitureIndex, id: Id) -> Option<(Self, Aabb3d)> {
		let extent = FurnitureCellExtent::from_id(id)?;
		let slots: Vec<_> = index
			.slots
			.iter()
			.filter(|slot| extent.contains_xz(slot.placement.translation))
			.cloned()
			.collect();
		if slots.is_empty() {
			return None;
		}
		let cell = FurnitureCell::new(extent, slots);
		let bounds = cell.bounds();
		Some((cell, bounds))
	}
}

pub(crate) fn same_slots(existing: &FurnitureCell, cell: &FurnitureCell) -> bool {
	slots_match(&existing.slots, &cell.slots)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cell::xz_radius_aabb;
	use bevy::math::Vec3;

	#[test]
	fn matching_slots_ignore_finish_seed() {
		let a = FurnitureNode::chair(Placement::IDENTITY).with_finish_seed(1);
		let b = FurnitureNode::chair(Placement::IDENTITY).with_finish_seed(9);
		assert!(slots_match(&[a.clone()], &[b]));
		let moved = FurnitureNode::chair(Placement::new(Vec3::X, 0.0));
		assert!(!slots_match(&[a.clone()], &[moved]));
		let turned = FurnitureNode::chair(Placement::new(Vec3::ZERO, 0.5));
		assert!(!slots_match(&[a.clone()], &[turned]));
		let scaled = FurnitureNode::chair(Placement::IDENTITY.with_scale(Vec3::splat(2.0)));
		assert!(!slots_match(&[a], &[scaled]));
	}

	#[test]
	fn original_ids_bin_slots_to_cells() {
		let mut index = FurnitureIndex::default();
		index.slots.push(FurnitureNode::chest(Placement::IDENTITY));
		index
			.slots
			.push(FurnitureNode::chair(Placement::new(Vec3::new(60.0, 0.0, 0.0), 0.0)));
		let region = xz_radius_aabb(Vec3::ZERO, 200.0);
		let ids = FurnitureCell::original_ids_for(&mut index, region);
		assert_eq!(ids.len(), 2);
	}

	#[test]
	fn original_ids_include_emptied_cells() {
		let mut index = FurnitureIndex::default();
		let extent = FurnitureCellExtent::from_cell_index(0, 0);
		let slot = FurnitureNode::chair(Placement::IDENTITY);
		let cell = FurnitureCell::new(extent, vec![slot]);
		let bounds = cell.bounds();
		index.insert(extent.id(), cell, bounds);
		index.slots.clear();
		let ids = FurnitureCell::original_ids_for(&mut index, xz_radius_aabb(Vec3::ZERO, 40.0));
		assert_eq!(ids, vec![OriginalId(extent.id())]);
	}

	#[test]
	fn generate_removes_emptied_cells() {
		let mut index = FurnitureIndex::default();
		let extent = FurnitureCellExtent::from_cell_index(0, 0);
		let slot = FurnitureNode::chair(Placement::IDENTITY);
		let cell = FurnitureCell::new(extent, vec![slot]);
		let bounds = cell.bounds();
		let region = xz_radius_aabb(Vec3::ZERO, 40.0);
		index.insert(extent.id(), cell, bounds);
		index.slots.clear();
		assert_eq!(index.generate_cells(region, 1), 1);
		assert!(index.get(extent.id()).is_none());
	}

	#[test]
	fn tracked_ids_find_cells_below_sea_level() {
		let mut index = FurnitureIndex::default();
		let extent = FurnitureCellExtent::from_cell_index(18, 36);
		let slot = FurnitureNode::chair(Placement::new(
			Vec3::new(extent.center().x, -141.0, extent.center().z),
			0.0,
		));
		let cell = FurnitureCell::new(extent, vec![slot]);
		let bounds = cell.bounds();
		index.insert(extent.id(), cell, bounds);
		let camera = xz_radius_aabb(Vec3::new(extent.center().x, -141.0, extent.center().z), 125.0);
		assert_eq!(SpatialIndex::<FurnitureCell>::tracked_ids_for(&index, camera).len(), 1);
		let sea = Aabb3d::from_min_max(
			Vec3::new(extent.min.x, 0.0, extent.min.z),
			Vec3::new(extent.max.x, 1.0, extent.max.z),
		);
		assert_eq!(SpatialIndex::<FurnitureCell>::tracked_ids_for(&index, sea).len(), 1);
	}

	#[test]
	fn refresh_slots_keys_the_exact_region_not_its_center_cell() {
		let mut index = FurnitureIndex::default();
		let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		let edge = FurnitureNode::chair(Placement::new(Vec3::new(240.0, 0.0, 0.0), 0.0));
		let first = xz_radius_aabb(Vec3::ZERO, 250.0);
		let mut expands = 0u32;
		index.refresh_slots(vec![(id, Version(1))], first, |_| {
			expands += 1;
			vec![edge.clone()]
		});
		assert_eq!(expands, 1);
		assert_eq!(index.slots.len(), 1);

		index.refresh_slots(vec![(id, Version(1))], first, |_| {
			expands += 1;
			panic!("identical region and fingerprint must not expand");
		});
		assert_eq!(expands, 1);

		let same_center_cell = xz_radius_aabb(Vec3::new(-20.0, 0.0, 0.0), 250.0);
		index.refresh_slots(vec![(id, Version(1))], same_center_cell, |_| {
			expands += 1;
			panic!("re-filter must use the cached development slots");
		});
		assert_eq!(expands, 1);
		assert!(
			index.slots.is_empty(),
			"a 50 m center-cell key would keep the slot that left the exact region"
		);
	}

	#[test]
	fn refresh_slots_evicts_departed_developments() {
		let mut index = FurnitureIndex::default();
		let keep = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		let leave = Id::from_cell(Aabb3d::from_min_max(Vec3::splat(10.0), Vec3::splat(11.0)));
		let region = xz_radius_aabb(Vec3::ZERO, 40.0);
		index.refresh_slots(vec![(keep, Version(1)), (leave, Version(1))], region, |_| {
			vec![FurnitureNode::chair(Placement::IDENTITY)]
		});
		assert_eq!(index.development_slots.len(), 2);
		index.refresh_slots(vec![(keep, Version(1))], region, |_| {
			panic!("kept development is already cached")
		});
		assert_eq!(index.development_slots.len(), 1);
		assert!(index.development_slots.contains_key(&keep));
	}
}
