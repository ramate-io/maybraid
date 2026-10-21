//! Player handoff after a matching actor is constructed.
//!
//! Construction does not choose a local player or change cameras. Handoff
//! requests are correlated to the same arrival identity.

use bevy::prelude::*;
use maybraid_spawns_core::SpawnIdentity;

/// Assign input, follow camera, and close arrival UI for this arrival.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivatePlayerArrival {
	pub identity: SpawnIdentity,
	pub actor: Entity,
}

impl ActivatePlayerArrival {
	pub const fn new(identity: SpawnIdentity, actor: Entity) -> Self {
		Self { identity, actor }
	}

	pub fn matches(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// The arrival finished and its UI should close.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerArrivalCompleted {
	pub identity: SpawnIdentity,
	pub actor: Entity,
}

impl PlayerArrivalCompleted {
	pub const fn new(identity: SpawnIdentity, actor: Entity) -> Self {
		Self { identity, actor }
	}

	pub fn matches(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// Construction failed. Preserve picker / arrival state and expose retry.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerArrivalFailed {
	pub identity: SpawnIdentity,
	pub retryable: bool,
}

impl PlayerArrivalFailed {
	pub const fn new(identity: SpawnIdentity, retryable: bool) -> Self {
		Self { identity, retryable }
	}
}
