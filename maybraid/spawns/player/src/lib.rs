//! Player arrival controller and UI-owned spawn picker.
//!
//! A controller decides what should happen and emits a request. World
//! adapters interpret first-wave gates and map presentation. Construction
//! waits for a matching placement-ready outcome; handoff consumes
//! [`ActorSpawned`] without this crate depending on placement or actors.

pub mod arrival;
pub mod controller;
pub mod handoff;
pub mod identity;
pub mod outcome;
pub mod picker;
pub mod request;

pub use arrival::{
	map_flick_dir, menu_nav_dir, nearest_candidate, nearest_in_direction,
	next_candidate_in_direction, reset_first_spawn_offer, ArrivalPhase, PendingPlayerRespawn,
	WorldPlayerRespawnConfig, WorldPlayerRespawnState,
};
pub use controller::{begin_player_arrivals, PlayerArrivalPolicy};
pub use handoff::{
	complete_player_handoffs, ActivatePlayerArrival, ActorSpawned, ArrivalActorFailed,
	ArrivalActorReady, PlayerArrivalCompleted, PlayerArrivalFailed,
};
pub use identity::{PlacementAttempt, SpawnIdentity, SpawnOwner, SpawnRequestId, SpawnSessionId};
pub use outcome::{SpawnCancelReason, SpawnFailureReason, SpawnOutcome};
pub use picker::{
	dispatch_spawn_picker, refresh_spawn_picker_focus, release_spawn_picker, SpawnPickerCapture,
	SpawnPickerController, SpawnPickerSheet,
};
pub use request::{
	ArrivalDestination, ArrivalReason, BeginPlayerArrival, DestinationSelected,
	PlayerChoseRespawnPoi,
};
pub use world_player::PlayerLifeSet;

use bevy::prelude::*;
use maybraid_controllers_core::CaptureResolveSystems;
use maybraid_input::MenuNavPad;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerHandoffSystems {
	Activate,
}

#[derive(Resource, Default)]
struct PlayerSpawnRuntime;

/// Headless-safe arrival contracts, picker, and session identity.
///
/// Idempotent. World adds first-wave / map / construction adapters beside this.
pub fn register_player_spawn_runtime(app: &mut App) {
	if app.world().contains_resource::<PlayerSpawnRuntime>() {
		return;
	}
	app.init_resource::<PlayerSpawnRuntime>()
		.init_resource::<WorldPlayerRespawnConfig>()
		.init_resource::<WorldPlayerRespawnState>()
		.init_resource::<SpawnPickerSheet>()
		.init_resource::<SpawnPickerController>()
		.init_resource::<SpawnPickerCapture>()
		.init_resource::<MenuNavPad>()
		.add_message::<BeginPlayerArrival>()
		.add_message::<DestinationSelected>()
		.add_message::<PlayerChoseRespawnPoi>()
		.add_message::<ActorSpawned>()
		.add_message::<ArrivalActorFailed>()
		.add_message::<ActivatePlayerArrival>()
		.add_message::<PlayerArrivalCompleted>()
		.add_message::<PlayerArrivalFailed>()
		.add_systems(Update, begin_player_arrivals)
		.add_systems(Update, complete_player_handoffs.in_set(PlayerHandoffSystems::Activate));
}

/// Arrival controller + UI-owned picker. World keeps map and construction
/// adapters and registers the picker systems beside the locked map.
pub struct PlayerSpawnPlugin;

impl Plugin for PlayerSpawnPlugin {
	fn build(&self, app: &mut App) {
		register_player_spawn_runtime(app);
		app.add_systems(Update, refresh_spawn_picker_focus.in_set(CaptureResolveSystems))
			.add_systems(Update, dispatch_spawn_picker.after(begin_player_arrivals));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn register_player_spawn_runtime_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_player_spawn_runtime(&mut app);
		register_player_spawn_runtime(&mut app);
		assert!(app.world().contains_resource::<WorldPlayerRespawnState>());
	}
}
