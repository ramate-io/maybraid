//! Outdoor ground and hold-above-base elevation. Uses composed height when
//! published; never treats raw base noise as a safe floor.

use bevy::prelude::*;
use durham::DurhamSurface;
use world_player::{player_position_above_surface, player_spawn_point_at, surface_or_hold_at};

use crate::request::{ActorClearance, PlacementPolicy};

/// A resolved pose before support is released.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementPose {
	pub position: Vec3,
	pub policy: PlacementPolicy,
}

impl PlacementPose {
	pub fn outdoor_at(surface: &DurhamSurface, xz: Vec2, _clearance: ActorClearance) -> Self {
		Self {
			position: player_spawn_point_at(xz, surface_or_hold_at(surface, xz)),
			policy: PlacementPolicy::OutdoorGround,
		}
	}

	pub fn interior_at(floor: Vec3) -> Self {
		Self {
			position: player_position_above_surface(floor),
			policy: PlacementPolicy::InteriorFloor,
		}
	}

	pub fn airborne_at(position: Vec3) -> Self {
		Self { position, policy: PlacementPolicy::Airborne }
	}

	pub const fn interior(self) -> bool {
		self.policy.interior()
	}
}
