//! POI destination → floor / clearance policy. An unresolved interior floor
//! is pending or failed, never a silent coordinate.

use bevy::prelude::*;
use poi_intelligence::PoiRecord;

use crate::request::PlacementPolicy;

impl PlacementPolicy {
	pub fn for_poi(record: &PoiRecord) -> Self {
		if record.support.stands_on_floor() {
			Self::InteriorFloor
		} else {
			Self::OutdoorGround
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Entity;
	use poi_intelligence::{PoiId, PoiKind, PoiRecord, PoiSupport};

	fn record(kind: PoiKind, local: bool, support: PoiSupport, y: f32) -> PoiRecord {
		PoiRecord {
			id: PoiId(1),
			entity: Entity::from_bits(1),
			kind,
			position: Vec3::new(8.0, y, 5.0),
			arrival_radius: 4.0,
			support,
			salience: 1.0,
			local,
			global: false,
		}
	}

	#[test]
	fn floor_support_keeps_its_authored_height_independent_of_kind() {
		let record = record(PoiKind::new("outdoor/label"), false, PoiSupport::Floor, 12.0);
		assert_eq!(PlacementPolicy::for_poi(&record), PlacementPolicy::InteriorFloor);
	}

	#[test]
	fn terrain_support_stays_outdoor_independent_of_kind() {
		let record = record(PoiKind::new("mobs/local"), true, PoiSupport::Terrain, 4.0);
		assert_eq!(PlacementPolicy::for_poi(&record), PlacementPolicy::OutdoorGround);
	}
}
