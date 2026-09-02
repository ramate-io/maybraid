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

	/// XZ disk plus a vertical band so storey changes are not “arrived” from below.
	pub fn contains(self, other: Vec3) -> bool {
		self.contains_xz(other) && (other.y - self.point.y).abs() <= self.vertical_slop()
	}

	/// Whether `other` crossed the XZ plane through this point along the segment
	/// from `segment_start`, while remaining inside a bounded lateral corridor.
	pub fn crossed_xz_from(self, segment_start: Vec3, other: Vec3, corridor: f32) -> bool {
		if (other.y - self.point.y).abs() > self.vertical_slop() {
			return false;
		}
		let incoming = self.point.xz() - segment_start.xz();
		let length = incoming.length();
		if length <= 1e-4 {
			return false;
		}
		let offset = other.xz() - self.point.xz();
		let crossed_plane = offset.dot(incoming) >= 0.0;
		let lateral_distance = offset.perp_dot(incoming).abs() / length;
		crossed_plane && lateral_distance <= corridor.max(0.0)
	}

	/// Whether `other` has started along the segment from this point toward `next`
	/// and remains inside a bounded lateral corridor.
	pub fn following_xz_toward(self, next: Vec3, other: Vec3, corridor: f32) -> bool {
		if (other.y - self.point.y).abs() > self.vertical_slop() {
			return false;
		}
		let outgoing = next.xz() - self.point.xz();
		let length = outgoing.length();
		if length <= 1e-4 {
			return false;
		}
		let offset = other.xz() - self.point.xz();
		let t = offset.dot(outgoing) / (length * length);
		let lateral_distance = offset.perp_dot(outgoing).abs() / length;
		t > 0.02 && t <= 1.0 && lateral_distance <= corridor.max(0.0)
	}

	pub fn vertical_slop(self) -> f32 {
		(self.radius + 0.45).max(0.7)
	}

	/// Horizontal plus vertical distance used for stuck detection.
	pub fn approach_distance(self, other: Vec3) -> f32 {
		let xz = self.xz_distance(other);
		let dy = (other.y - self.point.y).abs();
		xz.max(dy)
	}

	pub fn with_y(self, y: f32) -> Self {
		Self { point: Vec3::new(self.point.x, y, self.point.z), radius: self.radius }
	}

	pub fn with_radius(self, radius: f32) -> Self {
		Self { point: self.point, radius }
	}

	/// Flattened wish from `from` toward this point (y = 0).
	pub fn xz_wish_from(self, from: Vec3) -> Vec3 {
		Vec3::new(self.point.x - from.x, 0.0, self.point.z - from.z).normalize_or_zero()
	}

	/// `count` points on a circle in the xz plane, at height `y`.
	pub fn ring_around(center: Vec3, y: f32, radius: f32, count: u32, arrival: f32) -> Vec<Self> {
		let count = count.max(1);
		(0..count)
			.map(|i| {
				let yaw = i as f32 / count as f32 * std::f32::consts::TAU;
				Self::new(
					Vec3::new(center.x + radius * yaw.cos(), y, center.z + radius * yaw.sin()),
					arrival,
				)
			})
			.collect()
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

	#[test]
	fn contains_rejects_another_storey() -> anyhow::Result<()> {
		let loc = MovementLocation::new(Vec3::new(1.0, 4.0, 0.0), 0.5);
		assert!(loc.contains(Vec3::new(1.1, 4.2, 0.0)));
		assert!(!loc.contains(Vec3::new(1.1, 0.5, 0.0)));
		Ok(())
	}

	#[test]
	fn xz_wish_is_horizontal() -> anyhow::Result<()> {
		let loc = MovementLocation::new(Vec3::new(4.0, 9.0, 0.0), 0.5);
		let wish = loc.xz_wish_from(Vec3::ZERO);
		assert!((wish.x - 1.0).abs() < 1e-4, "{wish}");
		assert!(wish.y.abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn crossed_xz_requires_the_plane_corridor_and_height() -> anyhow::Result<()> {
		let loc = MovementLocation::new(Vec3::X, 0.1);
		assert!(loc.crossed_xz_from(Vec3::ZERO, Vec3::new(1.2, 0.0, 0.2), 0.4));
		assert!(!loc.crossed_xz_from(Vec3::ZERO, Vec3::new(1.2, 0.0, 0.8), 0.4));
		assert!(!loc.crossed_xz_from(Vec3::ZERO, Vec3::new(0.8, 0.0, 0.0), 0.4));
		assert!(!loc.crossed_xz_from(Vec3::ZERO, Vec3::new(1.2, 2.0, 0.0), 0.4));
		Ok(())
	}

	#[test]
	fn following_xz_requires_the_outgoing_segment_and_corridor() -> anyhow::Result<()> {
		let loc = MovementLocation::new(Vec3::X, 0.1);
		let next = Vec3::new(1.0, 0.0, 2.0);
		assert!(loc.following_xz_toward(next, Vec3::new(1.1, 0.0, 0.4), 0.4));
		assert!(!loc.following_xz_toward(next, Vec3::new(1.2, 0.0, -0.4), 0.4));
		assert!(!loc.following_xz_toward(next, Vec3::new(1.6, 0.0, 0.4), 0.4));
		assert!(!loc.following_xz_toward(next, Vec3::new(1.1, 2.0, 0.4), 0.4));
		Ok(())
	}

	#[test]
	fn ring_around_has_requested_count() -> anyhow::Result<()> {
		let points = MovementLocation::ring_around(Vec3::ZERO, 1.0, 2.0, 8, 0.4);
		assert_eq!(points.len(), 8);
		assert!((points[0].xz_distance(Vec3::ZERO) - 2.0).abs() < 1e-4);
		Ok(())
	}
}
