//! Collision-support readiness. Presentation / physics integration owns
//! collider cooking; this crate only observes it.

use bevy::prelude::*;

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
}

/// World adapters write this for the column under the current placement.
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
