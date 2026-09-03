//! Terrain-placed Shepherds Village development.

use bevy::math::Vec2;
use bevy::transform::components::Transform;
use richmond_developments::{ShepherdsVillage, ShepherdsVillageBuilding};

use crate::cell::yaw_about_xz;

#[derive(Debug, Clone)]
pub struct ShepherdsVillageDevelopment {
	pub village: ShepherdsVillage,
}

impl ShepherdsVillageDevelopment {
	pub fn host_transform(building: &ShepherdsVillageBuilding) -> Transform {
		yaw_about_xz(Vec2::new(building.center_xz.x, building.center_xz.y), building.yaw)
	}
}
