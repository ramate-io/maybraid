//! Fitted Les Halles development stored under a development-cell id.

use bevy::math::bounding::Aabb3d;
use bevy::transform::components::Transform;
use richmond_developments::{MixedUseLesHallesDevelopment, PlacedBuilding};

use crate::cell::yaw_about_xz;

/// One selected Les Halles development, fitted to pad confines.
#[derive(Debug, Clone)]
pub struct LesHallesDevelopment {
	pub cell: Aabb3d,
	pub building: PlacedBuilding<MixedUseLesHallesDevelopment>,
}

impl LesHallesDevelopment {
	/// Host pose: yaw about \(+Y\) through the development-cell center.
	pub fn host_transform(&self) -> Transform {
		yaw_about_xz(self.building.center_xz, self.building.yaw)
	}
}
