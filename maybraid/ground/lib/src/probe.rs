//! Downward hit query used by character pitch and any other “stand on colliders” path.

use bevy::ecs::entity::Entity;
use bevy::math::Vec3;

/// Downward ground sample from an origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundHit {
	pub point: Vec3,
	pub normal: Vec3,
	pub entity: Entity,
}

/// Sample ground under a world-space origin.
///
/// Implementations typically cast a short downward ray and skip the querying
/// character’s own collider via `exclude`. The Avian backend walks Fixed
/// colliders in the column and returns the lowest standable hit, not the
/// first volume the ray enters.
pub trait ElevationProbe {
	fn hit_down(
		&mut self,
		origin: Vec3,
		max_distance: f32,
		exclude: &[Entity],
	) -> Option<GroundHit>;

	/// Height of the ground hit under `(x, from_y, z)`, if any.
	fn height_at(
		&mut self,
		x: f32,
		z: f32,
		from_y: f32,
		max_distance: f32,
		exclude: &[Entity],
	) -> Option<f32> {
		self.hit_down(Vec3::new(x, from_y, z), max_distance, exclude)
			.map(|hit| hit.point.y)
	}
}
