//! Thin actor construction interpreter.
//!
//! [`SpawnActor::realize`] calls existing player-body APIs. NPC population
//! keeps calling `character.spawn` directly. Construction is synchronous;
//! there is no pending-work tracker and no generic recipe framework.
//! The player arrival controller consumes [`ArrivalActorReady`].

pub mod realize;
pub mod request;

pub use request::{ActorSpawnFailed, ActorSpawned, SpawnActor};

use bevy::prelude::*;
use maybraid_spawns_player::{ArrivalActorFailed, ArrivalActorReady};
use world_player::CharacterLocomotion;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActorSpawnSystems {
	Realize,
}

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
		.add_systems(Update, realize_actors.in_set(ActorSpawnSystems::Realize));
}

/// Construct requested bodies and emit the player-owned ready outcome.
pub fn realize_actors(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	locomotion: Res<CharacterLocomotion>,
	mut requests: MessageReader<SpawnActor>,
	mut failed: MessageReader<ActorSpawnFailed>,
	mut constructed: MessageWriter<ActorSpawned>,
	mut ready: MessageWriter<ArrivalActorReady>,
	mut arrivals_failed: MessageWriter<ArrivalActorFailed>,
) {
	for failed in failed.read() {
		arrivals_failed.write(ArrivalActorFailed::new(failed.identity));
	}
	for request in requests.read() {
		let spawned =
			request.realize(&mut commands, &mut meshes, &mut materials, locomotion.as_ref());
		ready.write(ArrivalActorReady::new(spawned.identity, spawned.actor));
		constructed.write(spawned);
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
	use maybraid_spawns_player::{
		ActivatePlayerArrival, ArrivalReason, SpawnIdentity, SpawnOwner, SpawnRequestId,
		SpawnSessionId, WorldPlayerRespawnState,
	};
	use world_player::{AwaitingTerrainSurface, OffTerrainAnchor, Player, RespawnOrigin};

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
	fn realize_emits_arrival_ready_without_activating() -> anyhow::Result<()> {
		let mut world = construct_world();
		world.init_resource::<WorldPlayerRespawnState>();
		world.init_resource::<Messages<SpawnActor>>();
		world.init_resource::<Messages<ActorSpawned>>();
		world.init_resource::<Messages<ActorSpawnFailed>>();
		world.init_resource::<Messages<ArrivalActorReady>>();
		world.init_resource::<Messages<ArrivalActorFailed>>();
		world.init_resource::<Messages<ActivatePlayerArrival>>();
		let identity = world.resource_mut::<WorldPlayerRespawnState>().begin_arrival(
			ArrivalReason::FirstSpawn { at: Vec3::ZERO },
			RespawnOrigin::began(None, false),
			0.0,
		);
		world.write_message(SpawnActor::player_body(identity, Vec3::Y, false));
		world
			.run_system_once(realize_actors)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let ready: Vec<_> = world.resource_mut::<Messages<ArrivalActorReady>>().drain().collect();
		assert_eq!(ready.len(), 1);
		assert!(ready[0].matches(identity));
		assert!(world.resource_mut::<Messages<ActivatePlayerArrival>>().drain().next().is_none());
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_some());
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
