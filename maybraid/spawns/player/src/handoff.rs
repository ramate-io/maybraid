//! Player handoff after a matching actor is constructed.
//!
//! Construction does not choose a local player or change cameras. Handoff
//! requests are correlated to the same arrival identity.

use bevy::prelude::*;
use maybraid_spawns_actors::ActorSpawned;
use maybraid_spawns_core::SpawnIdentity;

use crate::arrival::{ArrivalPhase, WorldPlayerRespawnState};

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

	/// Complete a matching arrival exactly once. A stale or already-activated
	/// `ActorSpawned` produces nothing.
	pub fn from_spawned(
		spawned: ActorSpawned,
		state: &mut WorldPlayerRespawnState,
	) -> Option<Self> {
		if !state.complete_arrival(spawned.identity) {
			return None;
		}
		Some(Self::new(spawned.identity, spawned.actor))
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

	pub fn from_activation(activation: ActivatePlayerArrival) -> Self {
		Self::new(activation.identity, activation.actor)
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

	pub fn preserve_arrival(
		identity: SpawnIdentity,
		state: &mut WorldPlayerRespawnState,
	) -> Option<Self> {
		if !state.accepts_identity(identity) {
			return None;
		}
		if let Some(pending) = state.pending.as_mut() {
			pending.advance(ArrivalPhase::ChoosingDestination);
		}
		Some(Self::new(identity, true))
	}
}

/// Match constructed actors to the live arrival. Stale sessions are ignored.
pub fn handoff_player_arrivals(
	mut spawned: MessageReader<ActorSpawned>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut activate: MessageWriter<ActivatePlayerArrival>,
	mut completed: MessageWriter<PlayerArrivalCompleted>,
) {
	for spawned in spawned.read().copied() {
		if let Some(activation) = ActivatePlayerArrival::from_spawned(spawned, &mut state) {
			completed.write(PlayerArrivalCompleted::from_activation(activation));
			activate.write(activation);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_spawns_core::{SpawnOwner, SpawnRequestId, SpawnSessionId};
	use world_player::RespawnOrigin;

	use crate::request::ArrivalReason;

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
	}

	fn pending_state() -> WorldPlayerRespawnState {
		let mut state = WorldPlayerRespawnState::default();
		state.begin_arrival(
			ArrivalReason::FirstSpawn { at: Vec3::ZERO },
			RespawnOrigin::began(None, false),
			0.0,
		);
		state
	}

	#[test]
	fn a_matching_actor_completes_the_arrival_once() {
		let mut state = pending_state();
		let identity = state.pending_identity().expect("pending");
		let actor = Entity::from_bits(7);
		let spawned = ActorSpawned::new(identity, actor, true);
		let first = ActivatePlayerArrival::from_spawned(spawned, &mut state).expect("activate");
		assert_eq!(first.actor, actor);
		assert!(state.pending.is_none());
		assert_eq!(state.last_activated(), Some(identity));
		assert!(ActivatePlayerArrival::from_spawned(spawned, &mut state).is_none());
	}

	#[test]
	fn a_stale_actor_spawned_does_not_activate() {
		let mut state = pending_state();
		state.end_session();
		state.begin_arrival(
			ArrivalReason::Death { at: Vec3::Y },
			RespawnOrigin::began(None, false),
			0.0,
		);
		let stale = identity();
		assert!(ActivatePlayerArrival::from_spawned(
			ActorSpawned::new(stale, Entity::from_bits(3), false),
			&mut state
		)
		.is_none());
		assert!(state.pending.is_some());
	}

	#[test]
	fn construction_failure_keeps_the_arrival() {
		let mut state = pending_state();
		let identity = state.pending_identity().expect("pending");
		let failed = PlayerArrivalFailed::preserve_arrival(identity, &mut state).expect("retry");
		assert!(failed.retryable);
		assert!(state.pending.is_some());
	}
}
