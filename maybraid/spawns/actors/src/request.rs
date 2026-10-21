//! Actor construction requests. Realization does not choose a local player
//! or change cameras. There is no generic recipe framework: callers pass
//! existing appearance types into [`spawn_player_body`](world_player::spawn_player_body)
//! or call `character.spawn` themselves.

use bevy::prelude::*;
use characters::CharacterAppearance;
use maybraid_spawns_player::{PlacementAttempt, SpawnFailureReason, SpawnIdentity};

/// Construct a player body at a resolved pose. Activation is a later handoff.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SpawnActor {
	pub identity: SpawnIdentity,
	pub attempt: PlacementAttempt,
	pub position: Vec3,
	pub interior: bool,
	pub appearance: Option<CharacterAppearance>,
	pub dress: bool,
}

impl SpawnActor {
	pub const fn player_body(
		identity: SpawnIdentity,
		attempt: PlacementAttempt,
		position: Vec3,
		interior: bool,
	) -> Self {
		Self { identity, attempt, position, interior, appearance: None, dress: false }
	}

	pub fn with_appearance(mut self, appearance: CharacterAppearance, dress: bool) -> Self {
		self.appearance = Some(appearance);
		self.dress = dress;
		self
	}

	pub fn belongs_to(&self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}

	pub fn matches_attempt(&self, identity: SpawnIdentity, attempt: PlacementAttempt) -> bool {
		self.belongs_to(identity) && self.attempt == attempt
	}
}

/// Construction produced an actor entity. Dress is world-owned presentation.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorSpawned {
	pub identity: SpawnIdentity,
	pub attempt: PlacementAttempt,
	pub actor: Entity,
	pub dress: bool,
}

impl ActorSpawned {
	pub const fn new(
		identity: SpawnIdentity,
		attempt: PlacementAttempt,
		actor: Entity,
		dress: bool,
	) -> Self {
		Self { identity, attempt, actor, dress }
	}

	pub fn belongs_to(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// Construction failed. Controllers may retry; they must keep arrival state.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorSpawnFailed {
	pub identity: SpawnIdentity,
	pub attempt: PlacementAttempt,
	pub reason: SpawnFailureReason,
}

impl ActorSpawnFailed {
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

	pub const fn retryable(self) -> bool {
		true
	}
}
