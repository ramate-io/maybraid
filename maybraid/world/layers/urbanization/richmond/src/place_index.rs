//! Spatial membership of [`DiscoverablePlace`] entities.
//!
//! Geneva names nearby places. It must not walk every place entity to decide
//! whether the local set changed.

use std::collections::HashMap;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::*;

use crate::place::DiscoverablePlace;

/// Bucket span for nearby place queries.
pub const PLACE_INDEX_CELL_M: f32 = 64.0;
/// Transform quant used for membership (not for stored anchors).
pub const PLACE_INDEX_QUANT_M: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndexedPlace {
	pub entity: Entity,
	pub place: DiscoverablePlace,
	pub xz: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PlaceStamp {
	place: DiscoverablePlace,
	qx: i32,
	qz: i32,
}

/// Bucketed world XZ index of discoverable places.
#[derive(Resource, Clone, Debug, Default)]
pub struct DiscoverablePlaceIndex {
	records: HashMap<Entity, IndexedPlace>,
	stamps: HashMap<Entity, PlaceStamp>,
	cells: HashMap<(i32, i32), Vec<Entity>>,
	membership: u64,
	bootstrapped: bool,
}

impl DiscoverablePlaceIndex {
	pub fn membership_revision(&self) -> u64 {
		self.membership
	}

	pub fn get(&self, entity: Entity) -> Option<&IndexedPlace> {
		self.records.get(&entity)
	}

	pub fn overlapping(&self, region: Aabb3d) -> Vec<IndexedPlace> {
		let mut out = Vec::new();
		for cell in cells_overlapping(region) {
			let Some(entities) = self.cells.get(&cell) else {
				continue;
			};
			for entity in entities {
				let Some(record) = self.records.get(entity) else {
					continue;
				};
				if xz_contains(region, record.xz) {
					out.push(*record);
				}
			}
		}
		out
	}

	pub fn upsert(&mut self, entity: Entity, place: DiscoverablePlace, xz: Vec2) {
		let stamp = PlaceStamp { place, qx: quantize(xz.x), qz: quantize(xz.y) };
		if self.stamps.get(&entity) == Some(&stamp) {
			if let Some(record) = self.records.get_mut(&entity) {
				record.xz = xz;
			}
			return;
		}
		self.remove(entity);
		let cell = cell_of(xz);
		self.records.insert(entity, IndexedPlace { entity, place, xz });
		self.stamps.insert(entity, stamp);
		self.cells.entry(cell).or_default().push(entity);
		self.membership = self.membership.wrapping_add(1);
	}

	pub fn remove(&mut self, entity: Entity) -> Option<IndexedPlace> {
		let record = self.records.remove(&entity)?;
		self.stamps.remove(&entity);
		let cell = cell_of(record.xz);
		if let Some(entities) = self.cells.get_mut(&cell) {
			entities.retain(|candidate| *candidate != entity);
			if entities.is_empty() {
				self.cells.remove(&cell);
			}
		}
		self.membership = self.membership.wrapping_add(1);
		Some(record)
	}

	fn bootstrap(&mut self, places: impl IntoIterator<Item = (Entity, DiscoverablePlace, Vec2)>) {
		if self.bootstrapped {
			return;
		}
		for (entity, place, xz) in places {
			self.upsert(entity, place, xz);
		}
		self.bootstrapped = true;
	}
}

fn sync_discoverable_place_index(
	mut index: ResMut<DiscoverablePlaceIndex>,
	all: Query<(Entity, &DiscoverablePlace, &GlobalTransform)>,
	changed: Query<
		Entity,
		Or<(Changed<DiscoverablePlace>, Changed<GlobalTransform>, Added<DiscoverablePlace>)>,
	>,
	mut removed: RemovedComponents<DiscoverablePlace>,
) {
	for entity in removed.read() {
		index.remove(entity);
	}
	if !index.bootstrapped {
		index.bootstrap(all.iter().map(|(entity, place, transform)| {
			let world = transform.translation();
			(entity, *place, Vec2::new(world.x, world.z))
		}));
		return;
	}
	for entity in &changed {
		let Ok((entity, place, transform)) = all.get(entity) else {
			continue;
		};
		let world = transform.translation();
		index.upsert(entity, *place, Vec2::new(world.x, world.z));
	}
}

pub(crate) fn register_place_index(app: &mut App) {
	app.init_resource::<DiscoverablePlaceIndex>()
		.add_systems(Update, sync_discoverable_place_index);
}

fn cell_of(xz: Vec2) -> (i32, i32) {
	((xz.x / PLACE_INDEX_CELL_M).floor() as i32, (xz.y / PLACE_INDEX_CELL_M).floor() as i32)
}

fn cells_overlapping(region: Aabb3d) -> impl Iterator<Item = (i32, i32)> {
	let min_x = (region.min.x / PLACE_INDEX_CELL_M).floor() as i32;
	let min_z = (region.min.z / PLACE_INDEX_CELL_M).floor() as i32;
	let max_x = ((region.max.x - 1e-3).max(region.min.x) / PLACE_INDEX_CELL_M).floor() as i32;
	let max_z = ((region.max.z - 1e-3).max(region.min.z) / PLACE_INDEX_CELL_M).floor() as i32;
	(min_x..=max_x).flat_map(move |x| (min_z..=max_z).map(move |z| (x, z)))
}

fn xz_contains(region: Aabb3d, xz: Vec2) -> bool {
	xz.x >= region.min.x && xz.x < region.max.x && xz.y >= region.min.z && xz.y < region.max.z
}

fn quantize(value: f32) -> i32 {
	(value / PLACE_INDEX_QUANT_M).round() as i32
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::place::DiscoverablePlaceLabel;

	#[test]
	fn overlapping_is_local_and_quant_ignores_jitter() {
		let mut world = World::new();
		let near = world.spawn_empty().id();
		let far = world.spawn_empty().id();
		let mut index = DiscoverablePlaceIndex::default();
		let place = DiscoverablePlace::host(DiscoverablePlaceLabel::House, 8.0, 1.1);
		index.upsert(near, place, Vec2::new(12.0, 18.0));
		index.upsert(far, place, Vec2::new(4_000.0, 4_000.0));
		let first = index.membership_revision();
		index.upsert(near, place, Vec2::new(12.4, 18.3));
		assert_eq!(index.membership_revision(), first);
		assert!((index.get(near).expect("near").xz - Vec2::new(12.4, 18.3)).length() < 1e-3);

		let region = Aabb3d::from_min_max(
			bevy::math::Vec3::new(-50.0, -1.0, -50.0),
			bevy::math::Vec3::new(50.0, 1.0, 50.0),
		);
		let hits = index.overlapping(region);
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].entity, near);

		index.upsert(near, place, Vec2::new(40.0, 40.0));
		assert_ne!(index.membership_revision(), first);
	}
}
