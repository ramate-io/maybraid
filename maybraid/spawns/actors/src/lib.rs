//! Thin actor construction interpreter.
//!
//! [`SpawnActor::realize`] calls existing player-body APIs. NPC population
//! keeps calling `character.spawn` directly. Construction is synchronous;
//! there is no pending-work tracker and no generic recipe framework.

pub mod realize;
pub mod request;

pub use request::{ActorSpawnFailed, ActorSpawned, SpawnActor};

use bevy::prelude::*;
use maybraid_spawns_player::{ActivatePlayerArrival, WorldPlayerRespawnState};

#[derive(Resource, Default)]
struct ActorSpawnRuntime;

/// Headless-safe actor construction messages. Idempotent.
pub fn register_actor_spawn_runtime(app: &mut App) {
	if app.world().contains_resource::<ActorSpawnRuntime>() {
		return;
	}
	app.init_resource::<ActorSpawnRuntime>()
		.add_message::<SpawnActor>()
		.add_message::<ActorSpawned>()
		.add_message::<ActorSpawnFailed>()
		.add_message::<ActivatePlayerArrival>()
		.add_systems(Update, complete_player_handoffs);
}

/// Handoff consumes a matching [`ActorSpawned`]. Construction does not choose
/// a local player; activation is exactly-once on the arrival identity.
pub fn complete_player_handoffs(
	mut spawned: MessageReader<ActorSpawned>,
	mut failed: MessageReader<ActorSpawnFailed>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut activate: MessageWriter<ActivatePlayerArrival>,
	mut arrivals_failed: MessageWriter<maybraid_spawns_player::PlayerArrivalFailed>,
) {
	for failed in failed.read() {
		if let Some(outcome) = state.note_placement_failed(failed.identity) {
			arrivals_failed.write(outcome);
		}
	}
	for spawned in spawned.read() {
		if let Some(activation) =
			ActivatePlayerArrival::from_constructed(spawned.identity, spawned.actor, &mut state)
		{
			activate.write(activation);
		}
	}
}

pub struct ActorSpawnPlugin;

impl Plugin for ActorSpawnPlugin {
	fn build(&self, app: &mut App) {
		register_actor_spawn_runtime(app);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_spawns_player::{SpawnIdentity, SpawnOwner, SpawnRequestId, SpawnSessionId};
	use world_player::{AwaitingTerrainSurface, CharacterLocomotion, OffTerrainAnchor, Player};

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
	}

	fn construct_world() -> World {
		let mut world = World::new();
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<StandardMaterial>>();
		world.init_resource::<CharacterLocomotion>();
		world
	}

	#[test]
	fn register_actor_spawn_runtime_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_actor_spawn_runtime(&mut app);
		register_actor_spawn_runtime(&mut app);
		assert!(app.world().contains_resource::<ActorSpawnRuntime>());
	}

	#[test]
	fn realize_creates_a_body_without_choosing_a_player() -> anyhow::Result<()> {
		let mut world = construct_world();
		let spawned = world
			.run_system_once(
				|mut commands: Commands,
				 mut meshes: ResMut<Assets<Mesh>>,
				 mut materials: ResMut<Assets<StandardMaterial>>,
				 locomotion: Res<CharacterLocomotion>| {
					SpawnActor::player_body(identity(), Vec3::new(8.0, 4.0, 5.0), false).realize(
						&mut commands,
						&mut meshes,
						&mut materials,
						locomotion.as_ref(),
					)
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(spawned.belongs_to(identity()));
		assert!(!spawned.dress);
		assert_eq!(
			world.get::<Transform>(spawned.actor).map(|transform| transform.translation),
			Some(Vec3::new(8.0, 4.0, 5.0))
		);
		assert!(world.get::<AwaitingTerrainSurface>(spawned.actor).is_some());
		assert!(world.get::<OffTerrainAnchor>(spawned.actor).is_none());
		assert!(world.get::<Player>(spawned.actor).is_some());
		Ok(())
	}

	#[test]
	fn handoff_consumes_a_matching_actor_spawned() -> anyhow::Result<()> {
		use maybraid_spawns_player::{ArrivalReason, WorldPlayerRespawnState};
		use world_player::RespawnOrigin;

		let mut world = World::new();
		world.init_resource::<WorldPlayerRespawnState>();
		world.init_resource::<Messages<ActorSpawned>>();
		world.init_resource::<Messages<ActorSpawnFailed>>();
		world.init_resource::<Messages<ActivatePlayerArrival>>();
		world.init_resource::<Messages<maybraid_spawns_player::PlayerArrivalFailed>>();
		let identity = world.resource_mut::<WorldPlayerRespawnState>().begin_arrival(
			ArrivalReason::FirstSpawn { at: Vec3::ZERO },
			RespawnOrigin::began(None, false),
			0.0,
		);
		world.write_message(ActorSpawned::new(identity, Entity::from_bits(7), false));
		world
			.run_system_once(complete_player_handoffs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let activations: Vec<_> =
			world.resource_mut::<Messages<ActivatePlayerArrival>>().drain().collect();
		assert_eq!(activations.len(), 1);
		assert_eq!(activations[0].actor, Entity::from_bits(7));
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		world.write_message(ActorSpawned::new(identity, Entity::from_bits(8), false));
		world
			.run_system_once(complete_player_handoffs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource_mut::<Messages<ActivatePlayerArrival>>().drain().next().is_none());
		Ok(())
	}

	#[test]
	fn an_interior_pose_anchors_off_terrain() -> anyhow::Result<()> {
		let mut world = construct_world();
		let spawned = world
			.run_system_once(
				|mut commands: Commands,
				 mut meshes: ResMut<Assets<Mesh>>,
				 mut materials: ResMut<Assets<StandardMaterial>>,
				 locomotion: Res<CharacterLocomotion>| {
					SpawnActor::player_body(identity(), Vec3::new(8.0, 12.0, 5.0), true).realize(
						&mut commands,
						&mut meshes,
						&mut materials,
						locomotion.as_ref(),
					)
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<OffTerrainAnchor>(spawned.actor).map(|anchor| anchor.translation),
			Some(Vec3::new(8.0, 12.0, 5.0))
		);
		Ok(())
	}
}
