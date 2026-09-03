//! Fitted Les Halles development stored under a development-cell id.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::transform::components::Transform;
use richmond_developments::MixedUseLesHallesDevelopment;

use crate::cell::yaw_about_xz;

/// One selected Les Halles development, fitted to pad confines.
#[derive(Debug, Clone)]
pub struct LesHallesDevelopment {
	pub cell: Aabb3d,
	/// Discrete yaw applied at host spawn about the cell center.
	pub confines_yaw: f32,
	pub development: MixedUseLesHallesDevelopment,
}

impl LesHallesDevelopment {
	/// Host pose: yaw about \(+Y\) through the 100 m cell center.
	pub fn host_transform(&self) -> Transform {
		let min = bevy::math::Vec3::from(self.cell.min);
		let max = bevy::math::Vec3::from(self.cell.max);
		yaw_about_xz(Vec2::new((min.x + max.x) * 0.5, (min.z + max.z) * 0.5), self.confines_yaw)
	}
}
