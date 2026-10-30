//! Actor support policy and accepted contact geometry.

use bevy::prelude::*;
use characters::CharacterLocomotion;

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
	pub fn accepts_normal(self, normal: Vec3) -> bool {
		self.accepts_normal_with_locomotion(normal, &CharacterLocomotion::default())
	}

	pub fn accepts_normal_with_locomotion(
		self,
		normal: Vec3,
		locomotion: &CharacterLocomotion,
	) -> bool {
		locomotion.accepts_support_normal(normal)
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
