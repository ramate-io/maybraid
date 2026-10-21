//! Actor construction requests. Realization does not choose a local player
//! or change cameras. There is no generic recipe framework: callers pass
//! existing appearance types into [`spawn_player_body`](world_player::spawn_player_body)
//! or call `character.spawn` themselves.

use bevy::prelude::*;
use characters::CharacterAppearance;
use maybraid_spawns_player::{SpawnFailureReason, SpawnIdentity};

/// Construct a player body at a resolved pose. Activation is a later handoff.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SpawnActor {
	pub identity: SpawnIdentity,
	pub position: Vec3,
	pub interior: bool,
	pub appearance: Option<CharacterAppearance>,
	pub dress: bool,
}

impl SpawnActor {
	pub const fn player_body(identity: SpawnIdentity, position: Vec3, interior: bool) -> Self {
		Self { identity, position, interior, appearance: None, dress: false }
	}

	pub fn with_appearance(mut self, appearance: CharacterAppearance, dress: bool) -> Self {
		self.appearance = Some(appearance);
		self.dress = dress;
		self
	}

	pub fn belongs_to(&self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// Construction produced an actor entity. Returned from realize; emit a
/// message only when an independent consumer needs it.
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
