//! Player handoff after a matching actor is constructed.
//!
//! Construction does not choose a local player or change cameras. The actor
//! interpreter emits the player-owned [`ArrivalActorReady`] so this crate
//! does not depend on `spawns/actors`. The arrival controller consumes that
//! outcome exactly once and emits [`ActivatePlayerArrival`]. World adapters
//! interpret activation (motor, camera, picker).

use bevy::prelude::*;

use crate::arrival::{ArrivalPhase, WorldPlayerRespawnState};
use crate::identity::SpawnIdentity;

/// Construction finished for this arrival. The player controller owns the
/// next step: emit [`ActivatePlayerArrival`] or drop a stale body.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrivalActorReady {
	pub identity: SpawnIdentity,
	pub actor: Entity,
}

impl ArrivalActorReady {
	pub const fn new(identity: SpawnIdentity, actor: Entity) -> Self {
		Self { identity, actor }
	}

	pub fn matches(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

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
		actor: Entity,
		state: &mut WorldPlayerRespawnState,
	) -> Option<Self> {
		if !state.complete_arrival(identity) {
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
	mut ready: MessageReader<ArrivalActorReady>,
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
		if let Some(activation) =
			ActivatePlayerArrival::from_constructed(ready.identity, ready.actor, &mut state)
		{
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

	use crate::identity::{SpawnOwner, SpawnRequestId, SpawnSessionId};
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
		let first =
			ActivatePlayerArrival::from_constructed(identity, actor, &mut state).expect("activate");
		assert_eq!(first.actor, actor);
		assert!(state.pending.is_none());
		assert_eq!(state.last_activated(), Some(identity));
		assert!(ActivatePlayerArrival::from_constructed(identity, actor, &mut state).is_none());
	}

	#[test]
	fn a_stale_construction_does_not_activate() {
		let mut state = pending_state();
		state.end_session();
		state.begin_arrival(
			ArrivalReason::Death { at: Vec3::Y },
			RespawnOrigin::began(None, false),
			0.0,
		);
		assert!(
			ActivatePlayerArrival::from_constructed(identity(), Entity::from_bits(3), &mut state)
				.is_none()
		);
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

	#[test]
	fn registered_handoff_activates_a_matching_actor_once() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.init_resource::<WorldPlayerRespawnState>();
		world.init_resource::<Messages<ArrivalActorReady>>();
		world.init_resource::<Messages<ArrivalActorFailed>>();
		world.init_resource::<Messages<ActivatePlayerArrival>>();
		world.init_resource::<Messages<PlayerArrivalFailed>>();
		let identity = world.resource_mut::<WorldPlayerRespawnState>().begin_arrival(
			ArrivalReason::FirstSpawn { at: Vec3::ZERO },
			RespawnOrigin::began(None, false),
			0.0,
		);
		let actor = world.spawn_empty().id();
		world.write_message(ArrivalActorReady::new(identity, actor));
		world.run_system_once(complete_player_handoffs).expect("handoff");
		let activations: Vec<_> =
			world.resource_mut::<Messages<ActivatePlayerArrival>>().drain().collect();
		assert_eq!(activations.len(), 1);
		assert_eq!(activations[0].actor, actor);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		let stale = world.spawn_empty().id();
		world.write_message(ArrivalActorReady::new(identity, stale));
		world.run_system_once(complete_player_handoffs).expect("stale");
		assert!(world.resource_mut::<Messages<ActivatePlayerArrival>>().drain().next().is_none());
	}
}
