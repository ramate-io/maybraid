//! Explicit floor vs terrain support for POI arrival poses.
//!
//! Generation records which surface a pin stands on. Spawn consumers read that
//! contract instead of inferring it from [`crate::PoiKind`]. Live collider
//! readiness stays a presentation/physics gate; it is not part of this type.

use bevy::prelude::*;

/// Surface an arrival pose stands on.
///
/// [`Self::Terrain`] stands on the presented outdoor ground (padded urban
/// terrain in Discovery). [`Self::Floor`] trusts the pin's authored storey Y
/// and skips outdoor snap. Collider readiness is not part of this type.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PoiSupport {
	/// Outdoor composed terrain.
	#[default]
	Terrain,
	/// Authored interior floor.
	Floor,
}

impl PoiSupport {
	pub const fn stands_on_floor(self) -> bool {
		matches!(self, Self::Floor)
	}
}

/// Arrival pose plus the support it was authored against.
///
/// Radius is the pin's walk/plant disk. [`Self::destination`] keeps floor Y
/// and uses live presented outdoor height when that cell is published,
/// otherwise the generated pin Y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoiArrival {
	pub position: Vec3,
	pub radius: f32,
	pub support: PoiSupport,
}

impl PoiArrival {
	pub fn terrain(position: Vec3, radius: f32) -> Self {
		Self { position, radius, support: PoiSupport::Terrain }
	}

	pub fn floor(position: Vec3, radius: f32) -> Self {
		Self { position, radius, support: PoiSupport::Floor }
	}

	/// Outdoor pins take `terrain_y`. Floor pins keep authored Y.
	pub fn surface_y(self, terrain_y: f32) -> Vec3 {
		self.destination(Some(terrain_y))
	}

	/// Surface point for spawn. Outdoor pins use `presented_y` when the
	/// presented cell is published; otherwise they keep the generated Y.
	pub fn destination(self, presented_y: Option<f32>) -> Vec3 {
		match self.support {
			PoiSupport::Terrain => {
				Vec3::new(self.position.x, presented_y.unwrap_or(self.position.y), self.position.z)
			}
			PoiSupport::Floor => self.position,
		}
	}

	pub fn stands_on_floor(self) -> bool {
		self.support.stands_on_floor()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn terrain_pins_take_composed_height() {
		let arrival = PoiArrival::terrain(Vec3::new(8.0, 1.0, -4.0), 6.0);
		assert!(!arrival.stands_on_floor());
		assert_eq!(arrival.surface_y(40.0), Vec3::new(8.0, 40.0, -4.0));
		assert_eq!(arrival.destination(None), Vec3::new(8.0, 1.0, -4.0));
	}

	#[test]
	fn floor_pins_keep_authored_height() {
		let arrival = PoiArrival::floor(Vec3::new(8.0, 12.0, -4.0), 4.0);
		assert!(arrival.stands_on_floor());
		assert_eq!(arrival.surface_y(40.0), Vec3::new(8.0, 12.0, -4.0));
		assert_eq!(arrival.destination(Some(40.0)), Vec3::new(8.0, 12.0, -4.0));
	}
}
