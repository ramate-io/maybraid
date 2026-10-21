//! Actor construction requests. Realization does not choose a local player
//! or change cameras.

use bevy::prelude::*;
use characters::CharacterAppearance;
use maybraid_spawns_core::{SpawnFailureReason, SpawnIdentity};

/// How to dress the constructed body. Persistence owns the saved loadout;
/// this is only the recipe handed to construction.
#[derive(Clone, Debug, PartialEq)]
pub struct ActorRecipe {
	pub appearance: Option<CharacterAppearance>,
	pub dress: bool,
}

impl ActorRecipe {
	pub fn appearance(appearance: CharacterAppearance, dress: bool) -> Self {
		Self { appearance: Some(appearance), dress }
	}

	pub const fn species_default() -> Self {
		Self { appearance: None, dress: false }
	}
}

/// Construct an actor at a resolved pose. Activation is a later handoff.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SpawnActor {
	pub identity: SpawnIdentity,
	pub position: Vec3,
	pub interior: bool,
	pub recipe: ActorRecipe,
}

impl SpawnActor {
	pub const fn new(
		identity: SpawnIdentity,
		position: Vec3,
		interior: bool,
		recipe: ActorRecipe,
	) -> Self {
		Self { identity, position, interior, recipe }
	}

	pub fn belongs_to(&self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// Construction produced an actor entity. Handoff matches this identity.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorSpawned {
	pub identity: SpawnIdentity,
	pub actor: Entity,
	pub dress: bool,
}

impl ActorSpawned {
	pub const fn new(identity: SpawnIdentity, actor: Entity, dress: bool) -> Self {
		Self { identity, actor, dress }
	}

	pub fn belongs_to(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// Construction failed. Controllers may retry; they must keep arrival state.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorSpawnFailed {
	pub identity: SpawnIdentity,
	pub reason: SpawnFailureReason,
}

impl ActorSpawnFailed {
	pub const fn new(identity: SpawnIdentity, reason: SpawnFailureReason) -> Self {
		Self { identity, reason }
	}

	pub fn belongs_to(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}

	pub const fn retryable(self) -> bool {
		true
	}
}

/// Cancel in-flight construction for this identity.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CancelActorSpawn {
	pub identity: SpawnIdentity,
}

impl CancelActorSpawn {
	pub const fn new(identity: SpawnIdentity) -> Self {
		Self { identity }
	}
}
