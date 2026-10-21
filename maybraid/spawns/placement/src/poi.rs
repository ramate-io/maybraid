//! POI destination → floor / clearance policy. An unresolved interior floor
//! is pending or failed, never a silent coordinate.

use bevy::prelude::*;
use poi_intelligence::{PoiKind, PoiRecord};

use crate::request::PlacementPolicy;

/// House / storey / lounge pins live inside a building. Urban settlement
/// pins stay outdoor even when the registry marks them local.
pub fn interior_building_poi(record: &PoiRecord) -> bool {
	record.local && (record.kind == local_poi() || record.kind == saloon_poi())
}

pub fn local_poi() -> PoiKind {
	PoiKind::new("mobs/local")
}

pub fn saloon_poi() -> PoiKind {
	PoiKind::new("mobs/saloon")
}

pub fn urban_poi() -> PoiKind {
	PoiKind::new("mobs/urban")
}

impl PlacementPolicy {
	pub fn for_poi(record: &PoiRecord) -> Self {
		if interior_building_poi(record) { Self::InteriorFloor } else { Self::OutdoorGround }
	}

	pub fn preferred_position(self, record: &PoiRecord) -> Vec3 {
		record.position
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Entity;
	use poi_intelligence::{PoiId, PoiRecord};

	fn record(kind: PoiKind, local: bool, y: f32) -> PoiRecord {
		PoiRecord {
			id: PoiId(1),
			entity: Entity::from_bits(1),
			kind,
			position: Vec3::new(8.0, y, 5.0),
			arrival_radius: 4.0,
			salience: 1.0,
			local,
			global: false,
		}
	}

	#[test]
	fn a_local_room_is_an_interior_floor() {
		let record = record(local_poi(), true, 12.0);
		assert_eq!(PlacementPolicy::for_poi(&record), PlacementPolicy::InteriorFloor);
		assert_eq!(PlacementPolicy::for_poi(&record).preferred_position(&record).y, 12.0);
	}

	#[test]
	fn an_urban_pin_stays_outdoor_even_when_local() {
		let record = record(urban_poi(), true, 4.0);
		assert_eq!(PlacementPolicy::for_poi(&record), PlacementPolicy::OutdoorGround);
	}
}
