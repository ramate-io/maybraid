//! First spawn / death / resume policy. World adapters emit
//! [`BeginPlayerArrival`]; this controller records the arrival.

use bevy::prelude::*;
use maybraid_spawns_core::SpawnIdentity;

use crate::arrival::{WorldPlayerRespawnConfig, WorldPlayerRespawnState};
use crate::request::BeginPlayerArrival;

/// Consume session-owned arrival requests. A request from another session is
/// ignored so a late retirement cannot start a new life.
pub fn begin_player_arrivals(
	config: Res<WorldPlayerRespawnConfig>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut arrivals: MessageReader<BeginPlayerArrival>,
) {
	for request in arrivals.read().copied() {
		let _ = state.begin_from(request, config.delay_secs);
	}
}

/// Policy helpers for world adapters that decide whether to emit
/// [`BeginPlayerArrival`].
pub struct PlayerArrivalPolicy;

impl PlayerArrivalPolicy {
	pub fn skip_first_offer(
		start_at_set: bool,
		resume_pending: bool,
		has_resume_trail: bool,
	) -> bool {
		start_at_set || resume_pending || has_resume_trail
	}

	pub fn offer_first_spawn(
		gameplay: bool,
		already_offered: bool,
		pending: bool,
		first_wave_ready: bool,
		pick_first_spawn: bool,
	) -> bool {
		gameplay && !already_offered && !pending && first_wave_ready && pick_first_spawn
	}

	pub fn identity_after(
		state: &WorldPlayerRespawnState,
		issued: BeginPlayerArrival,
	) -> Option<SpawnIdentity> {
		state.pending_identity().filter(|identity| identity.owner.matches(issued.owner))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;
	use maybraid_spawns_core::{SpawnOwner, SpawnSessionId};
	use world_player::RespawnOrigin;

	use crate::request::ArrivalReason;

	#[test]
	fn a_foreign_session_cannot_begin_an_arrival() {
		let mut state = WorldPlayerRespawnState::default();
		let foreign = BeginPlayerArrival::new(
			SpawnOwner::new(SpawnSessionId::new(99)),
			ArrivalReason::Death { at: Vec3::ZERO },
			RespawnOrigin::began(None, false),
		);
		assert!(state.begin_from(foreign, 0.0).is_none());
		assert!(state.pending.is_none());
	}

	#[test]
	fn first_offer_waits_for_support_and_policy() {
		assert!(!PlayerArrivalPolicy::offer_first_spawn(true, false, false, false, true));
		assert!(!PlayerArrivalPolicy::offer_first_spawn(true, false, false, true, false));
		assert!(PlayerArrivalPolicy::offer_first_spawn(true, false, false, true, true));
		assert!(PlayerArrivalPolicy::skip_first_offer(true, false, false));
	}
}
