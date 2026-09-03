//! Fitted Les Halles development stored under a development-cell id.

use bevy::math::bounding::Aabb3d;
use richmond_developments::MixedUseLesHallesDevelopment;

/// One selected Les Halles development, fitted to pad confines.
#[derive(Debug, Clone)]
pub struct LesHallesDevelopment {
	pub cell: Aabb3d,
	pub development: MixedUseLesHallesDevelopment,
}
