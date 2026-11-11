//! Floor arrival pins own their footprint independently of the mob host.

use anyhow::{Result, anyhow};
use bevy::prelude::*;

use crate::{
	AGENT_SEPARATION, ArrivalDisk, DEFAULT_NEARBY_RADIUS, NearbyFallback, NearbyPlace, NearbyQuery,
	Poi, PoiId, PoiInterests, PoiKind, PoiRegistry, PoiSupport,
};

const FLOOR: PoiKind = PoiKind::new("test/bounded-floor-arrival");
const FLOOR_ID: PoiId = PoiId(77);

struct ArrivalFixture {
	registry: PoiRegistry,
	interests: PoiInterests,
	host: Vec3,
	pin: Vec3,
	radius: f32,
}

impl ArrivalFixture {
	fn new(radius: f32, support: PoiSupport) -> Result<Self> {
		let host = Vec3::new(-60.0, 4.0, -40.0);
		let pin = Vec3::new(35.0, 28.0, 18.0);
		let mut registry = PoiRegistry::default();
		registry.upsert(
			Entity::from_bits(77),
			Poi::new(FLOOR_ID, FLOOR).with_arrival_radius(radius).with_support(support),
			pin,
			true,
			false,
		)?;
		Ok(Self { registry, interests: PoiInterests::one(FLOOR), host, pin, radius })
	}

	fn place(&self, seed: u64, occupied: &[Vec3], separation: f32) -> Option<NearbyPlace> {
		self.registry.place_nearby_among(
			self.host,
			NearbyQuery::weighted(DEFAULT_NEARBY_RADIUS),
			&self.interests,
			&[],
			seed,
			NearbyFallback::HOST,
			occupied,
			separation,
		)
	}

	fn assert_on_floor(&self, placed: NearbyPlace) {
		assert_eq!(placed.poi, Some(FLOOR_ID));
		assert_eq!(placed.support, PoiSupport::Floor);
		assert_eq!(placed.position.y, self.pin.y);
		assert!(
			(placed.position.xz() - self.pin.xz()).length() <= self.radius + 1e-4,
			"arrival {:?} escaped authored disk at {:?} with radius {}",
			placed.position,
			self.pin,
			self.radius
		);
	}
}

#[test]
fn floor_offsets_preserve_authored_disks_and_exact_height_with_empty_occupancy() -> Result<()> {
	for radius in [0.0, 0.5, 1.5, 40.0, 200.0] {
		let fixture = ArrivalFixture::new(radius, PoiSupport::Floor)?;
		for seed in 0..64 {
			let placed = fixture
				.place(seed, &[], AGENT_SEPARATION)
				.ok_or_else(|| anyhow!("unoccupied floor blocked"))?;
			fixture.assert_on_floor(placed);
			assert_eq!(fixture.place(seed, &[], AGENT_SEPARATION), Some(placed));
			if radius == 0.0 {
				assert_eq!(placed.position, fixture.pin);
			}
		}
	}
	Ok(())
}

#[test]
fn unrelated_occupants_do_not_move_an_already_clear_floor_arrival_around_the_host() -> Result<()> {
	for radius in [1.5, 40.0] {
		let fixture = ArrivalFixture::new(radius, PoiSupport::Floor)?;
		for seed in 0..32 {
			let initial = fixture
				.place(seed, &[], AGENT_SEPARATION)
				.ok_or_else(|| anyhow!("unoccupied floor blocked"))?;
			let occupied = [fixture.host, fixture.host + Vec3::X * 5.0];
			assert_eq!(fixture.place(seed, &occupied, AGENT_SEPARATION), Some(initial));
			fixture.assert_on_floor(initial);
		}
	}
	Ok(())
}

#[test]
fn overlapping_floor_arrival_resamples_only_inside_the_selected_disk() -> Result<()> {
	let fixture = ArrivalFixture::new(4.0, PoiSupport::Floor)?;
	for seed in 0..32 {
		let initial = fixture
			.place(seed, &[], 1.0)
			.ok_or_else(|| anyhow!("unoccupied floor blocked"))?;
		let next = fixture
			.place(seed, &[initial.position], 1.0)
			.ok_or_else(|| anyhow!("floor has room away from one occupant"))?;
		fixture.assert_on_floor(next);
		assert!(next.position.xz().distance(initial.position.xz()) >= 1.0 - 1e-4);
		assert_eq!(fixture.place(seed, &[initial.position], 1.0), Some(next));
	}
	Ok(())
}

#[test]
fn fully_occupied_floor_waits_instead_of_returning_a_host_orbit_with_floor_metadata() -> Result<()>
{
	for radius in [0.0, 0.5] {
		let fixture = ArrivalFixture::new(radius, PoiSupport::Floor)?;
		for seed in 0..16 {
			assert_eq!(fixture.place(seed, &[fixture.pin], AGENT_SEPARATION), None);
		}
	}
	Ok(())
}

#[test]
fn terrain_offsets_and_crowded_host_separation_keep_the_legacy_policy() -> Result<()> {
	for radius in [0.0, 1.5, 40.0, 200.0] {
		let fixture = ArrivalFixture::new(radius, PoiSupport::Terrain)?;
		for seed in 0..16 {
			let initial = NearbyPlace {
				position: ArrivalDisk::new(fixture.pin, radius).offset(seed),
				poi: Some(FLOOR_ID),
				support: PoiSupport::Terrain,
			};
			assert_eq!(fixture.place(seed, &[], AGENT_SEPARATION), Some(initial));
			let occupied = [initial.position];
			let legacy = initial.clear_of(fixture.host, &occupied, AGENT_SEPARATION, seed);
			assert_eq!(fixture.place(seed, &occupied, AGENT_SEPARATION), Some(legacy));
		}
	}
	Ok(())
}
