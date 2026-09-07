//! Nearby POI pose used by death replacement (mobs and the world player).

use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::hash::{mix, unit_f32};
use crate::{NearbyFallback, NearbyQuery, PoiId, PoiInterests, PoiRecord, PoiRegistry};

const ARRIVAL_DISK_MIN: f32 = 2.0;
const ARRIVAL_DISK_MAX: f32 = 12.0;
const DISK_DISTANCE_SALT: u64 = 0x736f_6d65_706c_6179;
const DISK_ANGLE_SALT: u64 = 0x6572_5f72_6573_7061;
const RING_ANGLE_SALT: u64 = 0x776f_726c_6470_6c79;

/// Horizontal distance kept between pack-mates on plant and replace.
pub const AGENT_SEPARATION: f32 = 2.0;

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
	query: NearbyQuery,
	interests: Option<&PoiInterests>,
	previous: Option<PoiId>,
	seed: u64,
	fallback: NearbyFallback,
) -> NearbyPlace {
	let excluded = previous.as_ref().map(core::slice::from_ref).unwrap_or(&[]);
	place_nearby_among(registry, center, query, interests, excluded, seed, fallback, &[], 0.0)
}

/// [`place_nearby`] plus last-N exclusion and sibling occupancy.
pub fn place_nearby_among(
	registry: Option<&PoiRegistry>,
	center: Vec3,
	query: NearbyQuery,
	interests: Option<&PoiInterests>,
	excluded: &[PoiId],
	seed: u64,
	fallback: NearbyFallback,
	occupied: &[Vec3],
	min_separation: f32,
) -> NearbyPlace {
	let poi = registry.zip(interests).and_then(|(registry, interests)| {
		registry.choose_in(center, query, interests, excluded, seed)
	});
	let placed = match poi {
		Some(poi) => NearbyPlace { position: offset_in_arrival_disk(poi, seed), poi: Some(poi.id) },
		None => {
			NearbyPlace { position: offset_in_fallback_ring(center, seed, fallback), poi: None }
		}
	};
	placed.clear_of(center, occupied, min_separation, seed)
}

impl NearbyPlace {
	/// Orbit `center` until XZ is at least `min_separation` from every occupant.
	pub fn clear_of(
		self,
		center: Vec3,
		occupied: &[Vec3],
		min_separation: f32,
		seed: u64,
	) -> Self {
		if occupied.is_empty() || min_separation <= 0.0 {
			return self;
		}
		let current = (self.position.xz() - center.xz()).length().max(min_separation);
		for attempt in 0..16 {
			let radius = current + attempt as f32 * min_separation * 0.25;
			let candidate = center
				+ polar_xz(radius, mix(seed ^ DISK_ANGLE_SALT.wrapping_add(attempt as u64 + 1)));
			let candidate = Vec3::new(candidate.x, self.position.y, candidate.z);
			if Self::separated(candidate, occupied, min_separation) {
				return Self { position: candidate, poi: self.poi };
			}
		}
		self.push_off_nearest(occupied, min_separation, seed)
	}

	fn separated(position: Vec3, occupied: &[Vec3], min_separation: f32) -> bool {
		occupied.iter().all(|other| (position.xz() - other.xz()).length() >= min_separation - 1e-4)
	}

	fn push_off_nearest(self, occupied: &[Vec3], min_separation: f32, seed: u64) -> Self {
		let Some(other) = occupied.iter().copied().min_by(|a, b| {
			(self.position.xz() - a.xz()).length().total_cmp(&(self.position.xz() - b.xz()).length())
		}) else {
			return self;
		};
		let delta = self.position.xz() - other.xz();
		let dir = if delta.length_squared() < 1e-6 {
			Vec2::from_angle(unit_f32(mix(seed)) * TAU)
		} else {
			delta.normalize()
		};
		Self {
			position: Vec3::new(
				other.x + dir.x * min_separation,
				self.position.y,
				other.z + dir.y * min_separation,
			),
			poi: self.poi,
		}
	}
}

impl PoiRegistry {
	/// [`place_nearby`] with this registry and a required interest table.
	pub fn place_nearby(
		&self,
		center: Vec3,
		query: NearbyQuery,
		interests: &PoiInterests,
		previous: Option<PoiId>,
		seed: u64,
		fallback: NearbyFallback,
	) -> NearbyPlace {
		place_nearby(Some(self), center, query, Some(interests), previous, seed, fallback)
	}

	/// [`place_nearby_among`] with this registry and a required interest table.
	pub fn place_nearby_among(
		&self,
		center: Vec3,
		query: NearbyQuery,
		interests: &PoiInterests,
		excluded: &[PoiId],
		seed: u64,
		fallback: NearbyFallback,
		occupied: &[Vec3],
		min_separation: f32,
	) -> NearbyPlace {
		place_nearby_among(
			Some(self),
			center,
			query,
			Some(interests),
			excluded,
			seed,
			fallback,
			occupied,
			min_separation,
		)
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
	use crate::{Poi, PoiInterest, PoiKind, DEFAULT_NEARBY_RADIUS};

	const CAMP: PoiKind = PoiKind::new("test/place-camp");

	#[test]
	fn fallback_ring_keeps_center_height() {
		let center = Vec3::new(10.0, 4.0, -5.0);
		let placed = place_nearby(
			None,
			center,
			NearbyQuery::weighted(DEFAULT_NEARBY_RADIUS),
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
			NearbyQuery::weighted(DEFAULT_NEARBY_RADIUS),
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
			NearbyQuery::weighted(DEFAULT_NEARBY_RADIUS),
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
			NearbyQuery::weighted(DEFAULT_NEARBY_RADIUS),
			&interests,
			Some(PoiId(11)),
			42,
			NearbyFallback::new(4.0, 12.0),
		);
		assert_eq!(placed.poi, Some(PoiId(12)));
		Ok(())
	}

	#[test]
	fn nearest_beyond_min_skips_the_death_site() -> anyhow::Result<()> {
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
			Vec3::X * 80.0,
			true,
			false,
		)?;
		registry.upsert(
			Entity::from_bits(13),
			Poi::new(PoiId(13), CAMP).with_arrival_radius(8.0),
			Vec3::X * 120.0,
			true,
			false,
		)?;
		let interests = PoiInterests::new([PoiInterest::new(CAMP, 1.0)]);
		let placed = registry.place_nearby(
			Vec3::ZERO,
			NearbyQuery::nearest_beyond(DEFAULT_NEARBY_RADIUS, 60.0),
			&interests,
			None,
			42,
			NearbyFallback::new(60.0, 100.0),
		);
		assert_eq!(placed.poi, Some(PoiId(12)));
		Ok(())
	}

	#[test]
	fn nearest_beyond_min_falls_back_when_every_poi_is_too_close() -> anyhow::Result<()> {
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
			NearbyQuery::nearest_beyond(DEFAULT_NEARBY_RADIUS, 60.0),
			&interests,
			None,
			42,
			NearbyFallback::new(60.0, 100.0),
		);
		assert!(placed.poi.is_none());
		assert!((60.0 - 1e-4..=100.0 + 1e-4).contains(&placed.position.xz().length()));
		Ok(())
	}

	#[test]
	fn occupancy_keeps_two_fallback_plants_apart() {
		let first = place_nearby(
			None,
			Vec3::ZERO,
			NearbyQuery::weighted(DEFAULT_NEARBY_RADIUS),
			None,
			None,
			1,
			NearbyFallback::new(4.0, 4.0),
		);
		let second = place_nearby_among(
			None,
			Vec3::ZERO,
			NearbyQuery::weighted(DEFAULT_NEARBY_RADIUS),
			None,
			&[],
			1,
			NearbyFallback::new(4.0, 4.0),
			&[first.position],
			AGENT_SEPARATION,
		);
		assert!((first.position.xz() - second.position.xz()).length() >= AGENT_SEPARATION - 1e-3);
	}

	#[test]
	fn last_n_excluded_pois_leave_a_remaining_candidate() -> anyhow::Result<()> {
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
		registry.upsert(
			Entity::from_bits(13),
			Poi::new(PoiId(13), CAMP).with_arrival_radius(8.0),
			Vec3::X * -20.0,
			true,
			false,
		)?;
		let interests = PoiInterests::new([PoiInterest::new(CAMP, 1.0)]);
		let placed = registry.place_nearby_among(
			Vec3::ZERO,
			NearbyQuery::weighted(DEFAULT_NEARBY_RADIUS),
			&interests,
			&[PoiId(11), PoiId(12)],
			42,
			NearbyFallback::new(4.0, 12.0),
			&[],
			0.0,
		);
		assert_eq!(placed.poi, Some(PoiId(13)));
		Ok(())
	}
}
