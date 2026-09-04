//! Fitted ring-fort development stored under a development-cell id.

use bevy::math::bounding::Aabb3d;
use bevy::transform::components::Transform;
use richmond_developments::{PlacedBuilding, RingFort};

use crate::cell::yaw_about_xz;

/// One selected ring fort, fitted to pad confines.
#[derive(Debug, Clone)]
pub struct RingFortDevelopment {
	pub cell: Aabb3d,
	pub building: PlacedBuilding<RingFort>,
}

impl RingFortDevelopment {
	/// Host pose: yaw about \(+Y\) through the development-cell center.
	pub fn host_transform(&self) -> Transform {
		yaw_about_xz(self.building.center_xz, self.building.yaw)
	}
}
