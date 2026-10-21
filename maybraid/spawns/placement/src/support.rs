//! Collision-support readiness. Presentation / physics integration owns
//! collider cooking; this crate only observes it.

use bevy::prelude::*;

use crate::request::PlacementPolicy;
use crate::terrain::PlacementPose;

/// Whether the spawn column has usable collision and composed height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SupportReadiness {
	pub height_ready: bool,
	pub collider_ready: bool,
}

impl SupportReadiness {
	pub const fn new(height_ready: bool, collider_ready: bool) -> Self {
		Self { height_ready, collider_ready }
	}

	pub const fn ready(self) -> bool {
		self.height_ready && self.collider_ready
	}

	pub const fn pending(self) -> bool {
		!self.ready()
	}

	/// Interior / outdoor wait for both bits. Airborne is already placed.
	pub fn for_pose(pose: PlacementPose, height_ready: bool, collider_ready: bool) -> Self {
		if pose.policy.waits_for_support() {
			Self::new(height_ready, collider_ready)
		} else {
			Self::new(true, true)
		}
	}

	pub const fn for_policy(
		policy: PlacementPolicy,
		height_ready: bool,
		collider_ready: bool,
	) -> Self {
		if policy.waits_for_support() {
			Self::new(height_ready, collider_ready)
		} else {
			Self::new(true, true)
		}
	}
}

/// Snapshot written by world adapters while they observe each pending pose.
/// Production polling uses per-request readiness on [`crate::PendingPlacement`].
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlacementSupport {
	pub readiness: SupportReadiness,
}

impl PlacementSupport {
	pub const fn ready() -> Self {
		Self { readiness: SupportReadiness::new(true, true) }
	}

	pub const fn waiting() -> Self {
		Self { readiness: SupportReadiness::new(false, false) }
	}

	pub fn observe(&mut self, readiness: SupportReadiness) {
		self.readiness = readiness;
	}

	pub const fn is_ready(self) -> bool {
		self.readiness.ready()
	}
}
