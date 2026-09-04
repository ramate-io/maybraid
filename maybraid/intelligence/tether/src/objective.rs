//! Tether vs stalk relative to a live entity.

use bevy::prelude::*;
use movement_intelligence::{MovementLocation, MovementObjective};

/// Allowed stalking annulus around a subject.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StalkRadii {
	/// Remain outside this radius.
	without: f32,
	/// Remain inside this radius.
	within: f32,
}

impl StalkRadii {
	pub fn new(without: f32, within: f32) -> Self {
		let without = without.max(0.0);
		Self { without, within: within.max(without) }
	}

	pub fn contains(self, distance: f32) -> bool {
		distance >= self.without && distance <= self.within
	}

	pub fn without(self) -> f32 {
		self.without
	}

	pub fn within(self) -> f32 {
		self.within
	}

	fn nearest(self, distance: f32) -> f32 {
		distance.clamp(self.without, self.within)
	}
}

/// Follow policy. Tether stays inside a leash; stalk stays within an annulus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TetherObjective {
	Tether(Entity, f32),
	Stalk(Entity, StalkRadii),
}

impl TetherObjective {
	pub fn subject(self) -> Entity {
		match self {
			Self::Tether(entity, _) | Self::Stalk(entity, _) => entity,
		}
	}

	pub fn with_subject(self, subject: Entity) -> Self {
		match self {
			Self::Tether(_, radius) => Self::Tether(subject, radius),
			Self::Stalk(_, radii) => Self::Stalk(subject, radii),
		}
	}

	/// Horizontal work left before the objective is geometrically met.
	pub fn remaining(self, from: Vec3, subject: Vec3) -> f32 {
		let dist = xz(from, subject);
		match self {
			Self::Tether(_, radius) => (dist - radius.max(0.0)).max(0.0),
			Self::Stalk(_, radii) if dist < radii.without => radii.without - dist,
			Self::Stalk(_, radii) if dist > radii.within => dist - radii.within,
			Self::Stalk(_, _) => 0.0,
		}
	}

	pub fn movement_objective(self, from: Vec3, subject: Vec3) -> MovementObjective {
		match self {
			Self::Tether(_, radius) => {
				MovementObjective::Reach(MovementLocation::new(subject, radius.max(0.4)))
			}
			Self::Stalk(_, radii) => {
				let distance = xz(from, subject);
				let boundary = if distance < radii.without { radii.without } else { radii.within };
				MovementObjective::EdgeOf(MovementLocation::new(subject, boundary.max(0.4)))
			}
		}
	}

	/// Point handed to routing when remaining work is beyond the local horizon.
	pub fn route_point(self, from: Vec3, subject: Vec3) -> Vec3 {
		match self {
			Self::Tether(_, _) => subject,
			Self::Stalk(_, radii) => {
				let distance = xz(from, subject);
				ring_point(from, subject, radii.nearest(distance))
			}
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
