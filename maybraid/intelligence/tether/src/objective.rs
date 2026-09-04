//! Tether vs stalk relative to a live entity.

use bevy::prelude::*;
use movement_intelligence::{MovementLocation, MovementObjective};

/// Follow policy. Radius is the leash (tether) or standoff (stalk).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TetherObjective {
	Tether(Entity, f32),
	Stalk(Entity, f32),
}

impl TetherObjective {
	pub fn subject(self) -> Entity {
		match self {
			Self::Tether(entity, _) | Self::Stalk(entity, _) => entity,
		}
	}

	pub fn radius(self) -> f32 {
		match self {
			Self::Tether(_, radius) | Self::Stalk(_, radius) => radius.max(0.0),
		}
	}

	pub fn with_subject(self, subject: Entity) -> Self {
		match self {
			Self::Tether(_, radius) => Self::Tether(subject, radius),
			Self::Stalk(_, radius) => Self::Stalk(subject, radius),
		}
	}

	/// Horizontal work left before the objective is geometrically met.
	pub fn remaining(self, from: Vec3, subject: Vec3) -> f32 {
		let dist = xz(from, subject);
		let radius = self.radius();
		match self {
			Self::Tether(_, _) => (dist - radius).max(0.0),
			Self::Stalk(_, _) => (dist - radius).abs(),
		}
	}

	pub fn movement_objective(self, subject: Vec3) -> MovementObjective {
		let location = MovementLocation::new(subject, self.radius().max(0.4));
		match self {
			Self::Tether(_, _) => MovementObjective::Reach(location),
			Self::Stalk(_, _) => MovementObjective::EdgeOf(location),
		}
	}

	/// Point handed to routing when remaining work is beyond the local horizon.
	pub fn route_point(self, from: Vec3, subject: Vec3) -> Vec3 {
		match self {
			Self::Tether(_, _) => subject,
			Self::Stalk(_, _) => ring_point(from, subject, self.radius()),
		}
	}
}

pub(crate) fn xz(a: Vec3, b: Vec3) -> f32 {
	Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z))
}

/// Keep the follower's azimuth; sit on the standoff ring.
pub(crate) fn ring_point(from: Vec3, subject: Vec3, radius: f32) -> Vec3 {
	let delta = Vec2::new(from.x - subject.x, from.z - subject.z);
	let dir = if delta.length_squared() < 1e-8 { Vec2::X } else { delta.normalize() };
	Vec3::new(subject.x + dir.x * radius, subject.y, subject.z + dir.y * radius)
}
