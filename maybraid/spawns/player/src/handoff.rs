//! Player handoff after a matching actor is constructed.
//!
//! Construction does not choose a local player or change cameras. The actor
//! interpreter emits the player-owned [`ActorSpawned`] so this crate
//! does not depend on `spawns/actors`. The arrival controller consumes that
//! outcome exactly once and emits [`ActivatePlayerArrival`]. World adapters
//! interpret activation (motor, camera, picker).

use bevy::prelude::*;

use crate::arrival::{ArrivalPhase, WorldPlayerRespawnState};
use crate::identity::{PlacementAttempt, SpawnIdentity};

/// Construction finished for this arrival. The player controller owns the
/// next step: emit [`ActivatePlayerArrival`] or drop a stale body.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorSpawned {
	pub identity: SpawnIdentity,
	pub attempt: PlacementAttempt,
	pub actor: Entity,
}

impl ActorSpawned {
	pub const fn new(identity: SpawnIdentity, attempt: PlacementAttempt, actor: Entity) -> Self {
		Self { identity, attempt, actor }
	}

	pub fn matches(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}

	pub fn matches_attempt(self, identity: SpawnIdentity, attempt: PlacementAttempt) -> bool {
		self.matches(identity) && self.attempt == attempt
	}
}

/// Previous name for [`ActorSpawned`]. Kept so in-flight adapters compile
/// while the stack merges upward.
pub type ArrivalActorReady = ActorSpawned;

/// Construction failed. The arrival controller keeps picker state.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrivalActorFailed {
	pub identity: SpawnIdentity,
}

impl ArrivalActorFailed {
	pub const fn new(identity: SpawnIdentity) -> Self {
		Self { identity }
	}

	pub fn matches(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

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
	/// construction produces nothing.
	pub fn from_constructed(
		identity: SpawnIdentity,
		attempt: PlacementAttempt,
		actor: Entity,
		state: &mut WorldPlayerRespawnState,
	) -> Option<Self> {
		if !state.complete_arrival(identity, attempt) {
			return None;
		}
		Some(Self::new(identity, actor))
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

/// Consume a matching constructed actor exactly once. Stale bodies despawn.
pub fn complete_player_handoffs(
	mut ready: MessageReader<ActorSpawned>,
	mut failed: MessageReader<ArrivalActorFailed>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut activate: MessageWriter<ActivatePlayerArrival>,
	mut arrivals_failed: MessageWriter<PlayerArrivalFailed>,
	mut commands: Commands,
) {
	for failed in failed.read() {
		if let Some(outcome) = state.note_placement_failed(failed.identity) {
			arrivals_failed.write(outcome);
		}
	}
	for ready in ready.read() {
		if let Some(activation) = ActivatePlayerArrival::from_constructed(
			ready.identity,
			ready.attempt,
			ready.actor,
			&mut state,
		) {
			activate.write(activation);
		} else {
			commands.entity(ready.actor).despawn();
		}
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

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;
	use world_player::RespawnOrigin;

	use crate::request::ArrivalReason;

	fn pending_state() -> (WorldPlayerRespawnState, SpawnIdentity, PlacementAttempt) {
		let mut state = WorldPlayerRespawnState::default();
		let identity = state.begin_arrival(
			ArrivalReason::FirstSpawn { at: Vec3::ZERO },
			RespawnOrigin::began(None, false),
			0.0,
		);
		let attempt = state.begin_placement(identity).expect("attempt");
		(state, identity, attempt)
	}

	#[test]
	fn a_matching_actor_completes_the_arrival_once() {
		let (mut state, identity, attempt) = pending_state();
		let actor = Entity::from_bits(7);
		let first = ActivatePlayerArrival::from_constructed(identity, attempt, actor, &mut state)
			.expect("activate");
		assert_eq!(first.actor, actor);
		assert!(state.pending.is_none());
		assert_eq!(state.last_activated(), Some(identity));
		assert!(
			ActivatePlayerArrival::from_constructed(identity, attempt, actor, &mut state).is_none()
		);
	}

	#[test]
	fn a_stale_construction_does_not_activate() {
		let (mut state, identity, attempt) = pending_state();
		state.end_session();
		state.begin_arrival(
			ArrivalReason::Death { at: Vec3::Y },
			RespawnOrigin::began(None, false),
			0.0,
		);
		assert!(ActivatePlayerArrival::from_constructed(
			identity,
			attempt,
			Entity::from_bits(3),
			&mut state
		)
		.is_none());
		assert!(state.pending.is_some());
	}

	#[test]
	fn a_superseded_attempt_does_not_activate() {
		let (mut state, identity, first) = pending_state();
		let second = state.begin_placement(identity).expect("replacement");
		assert!(ActivatePlayerArrival::from_constructed(
			identity,
			first,
			Entity::from_bits(3),
			&mut state
		)
		.is_none());
		assert!(state.pending.is_some());
		assert_eq!(state.pending_attempt(), Some(second));
	}

	#[test]
	fn construction_failure_keeps_the_arrival() {
		let (mut state, identity, _) = pending_state();
		let failed = PlayerArrivalFailed::preserve_arrival(identity, &mut state).expect("retry");
		assert!(failed.retryable);
		assert!(state.pending.is_some());
	}

	#[test]
	fn registered_handoff_activates_a_matching_actor_once() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.init_resource::<WorldPlayerRespawnState>();
		world.init_resource::<Messages<ActorSpawned>>();
		world.init_resource::<Messages<ArrivalActorFailed>>();
		world.init_resource::<Messages<ActivatePlayerArrival>>();
		world.init_resource::<Messages<PlayerArrivalFailed>>();
		let (identity, attempt) = {
			let mut state = world.resource_mut::<WorldPlayerRespawnState>();
			let identity = state.begin_arrival(
				ArrivalReason::FirstSpawn { at: Vec3::ZERO },
				RespawnOrigin::began(None, false),
				0.0,
			);
			let attempt = state.begin_placement(identity).expect("attempt");
			(identity, attempt)
		};
		let actor = world.spawn_empty().id();
		world.write_message(ActorSpawned::new(identity, attempt, actor));
		world.run_system_once(complete_player_handoffs).expect("handoff");
		let activations: Vec<_> =
			world.resource_mut::<Messages<ActivatePlayerArrival>>().drain().collect();
		assert_eq!(activations.len(), 1);
		assert_eq!(activations[0].actor, actor);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		let stale = world.spawn_empty().id();
		world.write_message(ActorSpawned::new(identity, attempt, stale));
		world.run_system_once(complete_player_handoffs).expect("stale");
		assert!(world.resource_mut::<Messages<ActivatePlayerArrival>>().drain().next().is_none());
		assert!(world.get_entity(stale).is_err(), "unclaimed construction is cleaned up");
	}
}
