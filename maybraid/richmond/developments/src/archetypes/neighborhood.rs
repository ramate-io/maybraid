use bevy_math::bounding::Aabb3d;

use crate::ShepherdsVillageBuilding;

/// Detached homes with broad spacing and street-like rows.
#[derive(Debug, Clone, PartialEq)]
pub struct SuburbanHomes {
	pub bounds: Aabb3d,
	pub homes: Vec<ShepherdsVillageBuilding>,
	pub secondary_buildings: Vec<ShepherdsVillageBuilding>,
}

impl SuburbanHomes {
	pub fn buildings(&self) -> impl Iterator<Item = &ShepherdsVillageBuilding> {
		self.homes.iter().chain(&self.secondary_buildings)
	}
}
