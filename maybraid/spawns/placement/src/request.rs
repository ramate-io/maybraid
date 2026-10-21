//! Placement requests and outcomes. Collision-support readiness is not a
//! [`lod::hcsg::GenerationScheme`] concern.

use bevy::prelude::*;
use maybraid_spawns_player::{ArrivalDestination, SpawnFailureReason, SpawnIdentity};

/// How the actor should meet the world. Interior floors stay authored.
/// Outdoor ground waits for composed support. Airborne is deliberate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementPolicy {
	OutdoorGround,
	InteriorFloor,
	Airborne,
}

impl PlacementPolicy {
	/// Outdoor ground and interior floors both wait for their supporting
	/// collider. An interior coordinate does not prove the floor is ready.
	pub const fn waits_for_support(self) -> bool {
		matches!(self, Self::OutdoorGround | Self::InteriorFloor)
	}

	pub const fn interior(self) -> bool {
		matches!(self, Self::InteriorFloor)
	}
}

/// Capsule clearance used when lifting onto a floor or composed surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorClearance {
	pub height: f32,
}

impl ActorClearance {
	pub const HUMANOID: Self = Self { height: 1.0 };

	pub const fn new(height: f32) -> Self {
		Self { height }
	}

	/// Lift from a floor / composed surface to the capsule center.
	pub const fn lift(self) -> f32 {
		self.height * 0.5 + 0.5
	}

	pub fn capsule_half_extents(self) -> Vec3 {
		Vec3::new(0.4, self.height.max(0.1) * 0.5, 0.4)
	}

	/// True when the standing capsule at `at` overlaps `[min, max]`.
	pub fn overlaps_aabb(self, at: Vec3, min: Vec3, max: Vec3) -> bool {
		let half = self.capsule_half_extents();
		let cmin = at - half;
		let cmax = at + half;
		cmin.x <= max.x
			&& cmax.x >= min.x
			&& cmin.y <= max.y
			&& cmax.y >= min.y
			&& cmin.z <= max.z
			&& cmax.z >= min.z
	}
}

/// Resolve a destination into a pose and support requirement.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct ResolvePlacement {
	pub identity: SpawnIdentity,
	pub destination: ArrivalDestination,
	pub clearance: ActorClearance,
	pub fallback_at: Vec3,
}

impl ResolvePlacement {
	pub const fn new(
		identity: SpawnIdentity,
		destination: ArrivalDestination,
		clearance: ActorClearance,
		fallback_at: Vec3,
	) -> Self {
		Self { identity, destination, clearance, fallback_at }
	}

	pub fn belongs_to(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// A usable pose. `support_ready` is false while collision support is pending.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PlacementReady {
	pub identity: SpawnIdentity,
	pub position: Vec3,
	pub policy: PlacementPolicy,
	pub support_ready: bool,
}

impl PlacementReady {
	pub const fn new(
		identity: SpawnIdentity,
		position: Vec3,
		policy: PlacementPolicy,
		support_ready: bool,
	) -> Self {
		Self { identity, position, policy, support_ready }
	}

	pub fn belongs_to(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// Placement could not produce a usable pose.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacementFailed {
	pub identity: SpawnIdentity,
	pub reason: SpawnFailureReason,
}

impl PlacementFailed {
	pub const fn new(identity: SpawnIdentity, reason: SpawnFailureReason) -> Self {
		Self { identity, reason }
	}

	pub fn belongs_to(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// Cancel in-flight placement for this identity.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CancelPlacement {
	pub identity: SpawnIdentity,
}

impl CancelPlacement {
	pub const fn new(identity: SpawnIdentity) -> Self {
		Self { identity }
	}
}
