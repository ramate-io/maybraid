//! Nearby POI pose used by death replacement (mobs and the world player).

use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::hash::{mix, unit_f32};
use crate::{PoiId, PoiInterests, PoiRecord, PoiRegistry};

/// Default scan used by mob and world-player death replacement.
pub const DEFAULT_NEARBY_RADIUS: f32 = 160.0;

const ARRIVAL_DISK_MIN: f32 = 2.0;
const ARRIVAL_DISK_MAX: f32 = 12.0;
const DISK_DISTANCE_SALT: u64 = 0x736f_6d65_706c_6179;
const DISK_ANGLE_SALT: u64 = 0x6572_5f72_6573_7061;
const RING_ANGLE_SALT: u64 = 0x776f_726c_6470_6c79;

/// Horizontal ring around `center` when no nearby POI is available.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearbyFallback {
	pub min_radius: f32,
	pub max_radius: f32,
}

impl NearbyFallback {
	pub const fn new(min_radius: f32, max_radius: f32) -> Self {
		Self { min_radius, max_radius }
	}
}

/// Horizontal pose from [`place_nearby`]. `position.y` is the POI or center height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearbyPlace {
	pub position: Vec3,
	pub poi: Option<PoiId>,
}

/// Choose a nearby POI disk, or a fallback ring around `center`.
///
/// `previous` is excluded when another candidate exists. Missing registry or
/// interests skip the POI scan and use the fallback ring. Callers own vertical
/// snap (terrain, last body height).
pub fn place_nearby(
	registry: Option<&PoiRegistry>,
	center: Vec3,
	query_radius: f32,
	interests: Option<&PoiInterests>,
	previous: Option<PoiId>,
	seed: u64,
	fallback: NearbyFallback,
) -> NearbyPlace {
	let poi = registry.zip(interests).and_then(|(registry, interests)| {
		registry.choose_nearby(center, query_radius, interests, previous, seed)
	});
	match poi {
		Some(poi) => NearbyPlace { position: offset_in_arrival_disk(poi, seed), poi: Some(poi.id) },
		None => {
			NearbyPlace { position: offset_in_fallback_ring(center, seed, fallback), poi: None }
		}
	}
}

impl PoiRegistry {
	/// [`place_nearby`] with this registry and a required interest table.
	pub fn place_nearby(
		&self,
		center: Vec3,
		query_radius: f32,
		interests: &PoiInterests,
		previous: Option<PoiId>,
		seed: u64,
		fallback: NearbyFallback,
	) -> NearbyPlace {
		place_nearby(Some(self), center, query_radius, Some(interests), previous, seed, fallback)
	}
}

fn offset_in_arrival_disk(poi: PoiRecord, seed: u64) -> Vec3 {
	let radius = poi.arrival_radius.clamp(ARRIVAL_DISK_MIN, ARRIVAL_DISK_MAX);
	let distance = unit_f32(mix(seed ^ DISK_DISTANCE_SALT)).sqrt() * radius;
	poi.position + polar_xz(distance, mix(seed ^ DISK_ANGLE_SALT))
}

fn offset_in_fallback_ring(center: Vec3, seed: u64, fallback: NearbyFallback) -> Vec3 {
	let min = fallback.min_radius.min(fallback.max_radius).max(0.0);
	let max = fallback.min_radius.max(fallback.max_radius).max(0.0);
	let distance = min + unit_f32(seed) * (max - min);
	center + polar_xz(distance, mix(seed ^ RING_ANGLE_SALT))
}

fn polar_xz(distance: f32, angle_seed: u64) -> Vec3 {
	let angle = unit_f32(angle_seed) * TAU;
	Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Poi, PoiInterest, PoiKind};

	const CAMP: PoiKind = PoiKind::new("test/place-camp");

	#[test]
	fn fallback_ring_keeps_center_height() {
		let center = Vec3::new(10.0, 4.0, -5.0);
		let placed = place_nearby(
			None,
			center,
			DEFAULT_NEARBY_RADIUS,
			None,
			None,
			42,
			NearbyFallback::new(8.0, 16.0),
		);
		let distance = (placed.position - center).xz().length();
		assert!((8.0 - 1e-4..=16.0 + 1e-4).contains(&distance));
		assert_eq!(placed.position.y, center.y);
		assert!(placed.poi.is_none());
	}

	#[test]
	fn missing_registry_does_not_need_interests() {
		let placed = place_nearby(
			None,
			Vec3::ZERO,
			DEFAULT_NEARBY_RADIUS,
			None,
			None,
			42,
			NearbyFallback::new(4.0, 12.0),
		);
		assert!((4.0 - 1e-4..=12.0 + 1e-4).contains(&placed.position.xz().length()));
	}

	#[test]
	fn nearby_poi_stays_inside_the_arrival_disk() -> anyhow::Result<()> {
		let mut registry = PoiRegistry::default();
		registry.upsert(
			Entity::from_bits(11),
			Poi::new(PoiId(11), CAMP).with_arrival_radius(8.0),
			Vec3::X * 20.0,
			true,
			false,
		)?;
		let interests = PoiInterests::new([PoiInterest::new(CAMP, 1.0)]);
		let placed = registry.place_nearby(
			Vec3::ZERO,
			DEFAULT_NEARBY_RADIUS,
			&interests,
			None,
			42,
			NearbyFallback::new(8.0, 16.0),
		);
		assert_eq!(placed.poi, Some(PoiId(11)));
		assert!((placed.position.xz() - Vec2::X * 20.0).length() <= 8.0 + 1e-4);
		assert_eq!(placed.position.y, 0.0);
		Ok(())
	}

	#[test]
	fn nearby_choice_skips_the_previous_poi() -> anyhow::Result<()> {
		let mut registry = PoiRegistry::default();
		registry.upsert(
			Entity::from_bits(11),
			Poi::new(PoiId(11), CAMP).with_arrival_radius(8.0),
			Vec3::X * 20.0,
			true,
			false,
		)?;
		registry.upsert(
			Entity::from_bits(12),
			Poi::new(PoiId(12), CAMP).with_arrival_radius(8.0),
			Vec3::Z * 20.0,
			true,
			false,
		)?;
		let interests = PoiInterests::new([PoiInterest::new(CAMP, 1.0)]);
		let placed = registry.place_nearby(
			Vec3::ZERO,
			DEFAULT_NEARBY_RADIUS,
			&interests,
			Some(PoiId(11)),
			42,
			NearbyFallback::new(4.0, 12.0),
		);
		assert_eq!(placed.poi, Some(PoiId(12)));
		Ok(())
	}
}
