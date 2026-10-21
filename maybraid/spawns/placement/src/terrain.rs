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
	pub clearance: ActorClearance,
}

impl PlacementPose {
	pub fn outdoor_at(surface: &DurhamSurface, xz: Vec2, clearance: ActorClearance) -> Self {
		let floor_y = surface_or_hold_at(surface, xz);
		Self {
			position: player_spawn_point_at(xz, floor_y),
			policy: PlacementPolicy::OutdoorGround,
			clearance,
		}
	}

	pub fn interior_at(floor: Vec3, clearance: ActorClearance) -> Self {
		Self {
			position: player_position_above_surface(floor),
			policy: PlacementPolicy::InteriorFloor,
			clearance,
		}
	}

	pub fn airborne_at(position: Vec3, clearance: ActorClearance) -> Self {
		Self { position, policy: PlacementPolicy::Airborne, clearance }
	}

	pub const fn interior(self) -> bool {
		self.policy.interior()
	}

	/// Occupancy that is not the supporting floor fails clearance.
	pub fn reject_blocked(
		self,
		blocked: bool,
	) -> Result<Self, maybraid_spawns_player::SpawnFailureReason> {
		if blocked {
			Err(maybraid_spawns_player::SpawnFailureReason::Clearance)
		} else {
			Ok(self)
		}
	}
}
