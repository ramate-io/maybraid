//! Placement requests and outcomes. Collision-support readiness is not a
//! [`lod::hcsg::GenerationScheme`] concern.

use bevy::prelude::*;
use characters::LocomotionCapsule;
use maybraid_spawns_player::{
	ArrivalDestination, PlacementAttempt, SpawnFailureReason, SpawnIdentity,
};

/// How the actor should meet the world. Interior and outdoor share one
/// probe; only the vertical search interval differs.
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

	pub const fn support_policy(self) -> SupportPolicy {
		match self {
			Self::InteriorFloor => SupportPolicy::BuildingWalk,
			Self::OutdoorGround | Self::Airborne => SupportPolicy::FixedUpward,
		}
	}
}

/// Which colliders may become the accepted support. Roof / surface
/// eligibility is this filter plus the column bounds, not special-case
/// skip logic in the probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportPolicy {
	/// Any Fixed collider with an upward normal inside the column.
	FixedUpward,
	/// Same Fixed/upward filter; the tighter interior interval excludes
	/// roofs and lower terrain.
	BuildingWalk,
}

impl SupportPolicy {
	pub const UPWARD: f32 = 0.55;

	pub fn accepts_normal(self, normal: Vec3) -> bool {
		normal.y > Self::UPWARD
	}
}

/// Capsule clearance used when lifting onto a floor or composed surface.
/// Prefer [`LocomotionCapsule`] on new requests; this remains a thin
/// conversion helper for existing callers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorClearance {
	pub height: f32,
}

impl ActorClearance {
	pub const HUMANOID: Self = Self { height: LocomotionCapsule::HUMANOID.length };

	pub const fn new(height: f32) -> Self {
		Self { height }
	}

	pub const RADIUS: f32 = LocomotionCapsule::HUMANOID.radius;

	pub fn to_hull(self) -> LocomotionCapsule {
		LocomotionCapsule::HUMANOID.with_half_height(Self::RADIUS + self.height.max(0.05) * 0.5)
	}

	pub fn from_hull(hull: LocomotionCapsule) -> Self {
		Self { height: hull.length }
	}

	pub fn surface_lift(self) -> f32 {
		self.to_hull().spawn_height()
	}

	pub fn capsule_half_extents(self) -> Vec3 {
		let hull = self.to_hull();
		Vec3::new(hull.radius, hull.half_height(), hull.radius)
	}
}

/// Resolve a destination into a bounded vertical search column.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct ResolvePlacement {
	pub identity: SpawnIdentity,
	pub attempt: PlacementAttempt,
	pub destination: ArrivalDestination,
	pub hull: LocomotionCapsule,
	pub fallback_at: Vec3,
}

impl ResolvePlacement {
	pub const fn new(
		identity: SpawnIdentity,
		attempt: PlacementAttempt,
		destination: ArrivalDestination,
		hull: LocomotionCapsule,
		fallback_at: Vec3,
	) -> Self {
		Self { identity, attempt, destination, hull, fallback_at }
	}

	pub const fn humanoid(
		identity: SpawnIdentity,
		attempt: PlacementAttempt,
		destination: ArrivalDestination,
		fallback_at: Vec3,
	) -> Self {
		Self::new(identity, attempt, destination, LocomotionCapsule::HUMANOID, fallback_at)
	}

	pub fn belongs_to(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}

	pub fn matches_attempt(self, identity: SpawnIdentity, attempt: PlacementAttempt) -> bool {
		self.belongs_to(identity) && self.attempt == attempt
	}
}

/// Contact that became the accepted support. Construction may revalidate it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementSupportHit {
	pub entity: Entity,
	pub point: Vec3,
	pub normal: Vec3,
}

impl PlacementSupportHit {
	pub const fn new(entity: Entity, point: Vec3, normal: Vec3) -> Self {
		Self { entity, point, normal }
	}
}

/// A usable standing pose. Emitted only after the probe accepted a collider.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PlacementReady {
	pub identity: SpawnIdentity,
	pub attempt: PlacementAttempt,
	pub position: Vec3,
	pub policy: PlacementPolicy,
	pub support_ready: bool,
	pub support: Option<PlacementSupportHit>,
}

impl PlacementReady {
	pub const fn new(
		identity: SpawnIdentity,
		attempt: PlacementAttempt,
		position: Vec3,
		policy: PlacementPolicy,
		support_ready: bool,
	) -> Self {
		Self { identity, attempt, position, policy, support_ready, support: None }
	}

	pub const fn accepted(
		identity: SpawnIdentity,
		attempt: PlacementAttempt,
		position: Vec3,
		policy: PlacementPolicy,
		support: PlacementSupportHit,
	) -> Self {
		Self { identity, attempt, position, policy, support_ready: true, support: Some(support) }
	}

	pub fn belongs_to(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}

	pub fn matches_attempt(self, identity: SpawnIdentity, attempt: PlacementAttempt) -> bool {
		self.belongs_to(identity) && self.attempt == attempt
	}
}

/// Placement could not produce a usable pose.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacementFailed {
	pub identity: SpawnIdentity,
	pub attempt: PlacementAttempt,
	pub reason: SpawnFailureReason,
}

impl PlacementFailed {
	pub const fn new(
		identity: SpawnIdentity,
		attempt: PlacementAttempt,
		reason: SpawnFailureReason,
	) -> Self {
		Self { identity, attempt, reason }
	}

	pub fn belongs_to(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}

	pub fn matches_attempt(self, identity: SpawnIdentity, attempt: PlacementAttempt) -> bool {
		self.belongs_to(identity) && self.attempt == attempt
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
