//! A point plus a satisfaction radius.

use bevy::prelude::*;

/// World point the mover is at, or wants to treat as a region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementLocation {
	pub point: Vec3,
	/// Satisfied when the xz distance to `point` is ≤ this radius.
	pub radius: f32,
}

impl MovementLocation {
	pub fn new(point: Vec3, radius: f32) -> Self {
		Self { point, radius }
	}

	/// Horizontal distance from `point` to `other`.
	pub fn xz_distance(self, other: Vec3) -> f32 {
		Vec2::new(self.point.x, self.point.z).distance(Vec2::new(other.x, other.z))
	}

	/// Whether `other` is inside the xz disk.
	pub fn contains_xz(self, other: Vec3) -> bool {
		self.xz_distance(other) <= self.radius
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn contains_xz_ignores_height() -> anyhow::Result<()> {
		let loc = MovementLocation::new(Vec3::new(1.0, 8.0, 0.0), 0.5);
		assert!(loc.contains_xz(Vec3::new(1.2, 0.0, 0.1)));
		assert!(!loc.contains_xz(Vec3::new(2.0, 8.0, 0.0)));
		Ok(())
	}
}
