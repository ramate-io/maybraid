//! Generated 50 m furniture cells.

use std::collections::{HashMap, HashSet};

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use building_components::FurnitureNode;
use lod::gen::{GenerationScheme, Id, OriginalId, SpatialIndex, StorageStatus, TrackedId, Version};
use lod::lod_ref::LodRef;

use crate::cell::{intersects_xz, FurnitureCellExtent};
use crate::host::FurnitureCell;
use crate::slots::FurnishedDevelopment;

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
	slots_region_cell: Option<(i32, i32)>,
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
		self.slots_region_cell = None;
	}

	pub(crate) fn refresh_slots(&mut self, tracked: Vec<FurnishedDevelopment>, region: Aabb3d) {
		let mut fingerprint: Vec<_> = tracked.iter().map(|dev| (dev.id, dev.version)).collect();
		fingerprint.sort();
		let region_cell = FurnitureCellExtent::cell_index_containing(Vec3::new(
			(region.min.x + region.max.x) * 0.5,
			0.0,
			(region.min.z + region.max.z) * 0.5,
		));
		if fingerprint == self.slots_fingerprint && Some(region_cell) == self.slots_region_cell {
			return;
		}
		self.slots.clear();
		for development in tracked {
			let cached = self.development_slots.entry(development.id).or_insert_with(|| {
				CachedDevelopmentSlots { version: Version(0), slots: Vec::new() }
			});
			if cached.version != development.version {
				cached.slots = development.slots;
				cached.version = development.version;
			}
			self.slots.extend(
				cached
					.slots
					.iter()
					.filter(|slot| {
						let p = slot.placement.translation;
						p.x >= region.min.x
							&& p.x <= region.max.x
							&& p.z >= region.min.z
							&& p.z <= region.max.z
					})
					.cloned(),
			);
		}
		self.slots_fingerprint = fingerprint;
		self.slots_region_cell = Some(region_cell);
	}
}

fn slots_match(left: &[FurnitureNode], right: &[FurnitureNode]) -> bool {
	left.len() == right.len()
		&& left.iter().zip(right).all(|(a, b)| {
			a.geometry == b.geometry
				&& (a.placement.translation - b.placement.translation).length_squared() < 1e-4
		})
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

	fn insert(&mut self, id: Id, value: FurnitureCell, bounds: Aabb3d, _lod_ref: &LodRef) {
		let version = self.next_version();
		self.cells.insert(id, StoredFurnitureCell { value, bounds, version });
	}
}

impl GenerationScheme<FurnitureIndex> for FurnitureCell {
	fn original_ids_for(index: &mut FurnitureIndex, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids = HashSet::new();
		for slot in &index.slots {
			let p = slot.placement.translation;
			if p.x < region.min.x || p.x > region.max.x || p.z < region.min.z || p.z > region.max.z
			{
				continue;
			}
			ids.insert(
				FurnitureCellExtent::from_cell_index(
					FurnitureCellExtent::cell_index_containing(p).0,
					FurnitureCellExtent::cell_index_containing(p).1,
				)
				.id(),
			);
		}
		ids.into_iter().map(OriginalId).collect()
	}

	fn build_with_id(
		index: &mut FurnitureIndex,
		id: Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
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
	use building_components::Placement;

	#[test]
	fn matching_slots_ignore_finish_seed() {
		let a = FurnitureNode::chair(Placement::IDENTITY).with_finish_seed(1);
		let b = FurnitureNode::chair(Placement::IDENTITY).with_finish_seed(9);
		assert!(slots_match(&[a.clone()], &[b]));
		let moved = FurnitureNode::chair(Placement::new(Vec3::X, 0.0));
		assert!(!slots_match(&[a], &[moved]));
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
	fn tracked_ids_find_cells_below_sea_level() {
		let mut index = FurnitureIndex::default();
		let extent = FurnitureCellExtent::from_cell_index(18, 36);
		let slot = FurnitureNode::chair(Placement::new(
			Vec3::new(extent.center().x, -141.0, extent.center().z),
			0.0,
		));
		let cell = FurnitureCell::new(extent, vec![slot]);
		let bounds = cell.bounds();
		let identity = Transform::IDENTITY;
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		index.insert(extent.id(), cell, bounds, &lod_ref);
		let camera = xz_radius_aabb(Vec3::new(extent.center().x, -141.0, extent.center().z), 125.0);
		assert_eq!(SpatialIndex::<FurnitureCell>::tracked_ids_for(&index, camera).len(), 1);
		let sea = Aabb3d::from_min_max(
			Vec3::new(extent.min.x, 0.0, extent.min.z),
			Vec3::new(extent.max.x, 1.0, extent.max.z),
		);
		assert_eq!(SpatialIndex::<FurnitureCell>::tracked_ids_for(&index, sea).len(), 1);
	}
}
