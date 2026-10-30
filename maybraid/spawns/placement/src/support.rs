//! Player placement readiness snapshot; geometry lives in the shared leaf.

use bevy::prelude::*;
pub use maybraid_spawns_support::{column_presentation_ready, column_volume, SupportReadiness};

/// Snapshot written by tests. Production polling uses per-request
/// readiness on [`crate::PendingPlacement`].
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlacementSupport {
	pub readiness: SupportReadiness,
}

impl PlacementSupport {
	pub const fn ready() -> Self {
		Self { readiness: SupportReadiness::presented() }
	}

	pub const fn waiting() -> Self {
		Self { readiness: SupportReadiness::waiting() }
	}

	pub fn observe(&mut self, readiness: SupportReadiness) {
		self.readiness = readiness;
	}

	pub const fn is_ready(self) -> bool {
		self.readiness.ready()
	}
}
