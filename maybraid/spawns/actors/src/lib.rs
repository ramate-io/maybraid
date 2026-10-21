//! Thin actor construction interpreter.
//!
//! [`SpawnActor::realize`] calls existing player-body APIs. NPC population
//! keeps calling `character.spawn` directly. Construction is synchronous;
//! there is no pending-work tracker and no generic recipe framework.

pub mod realize;
pub mod request;

pub use request::{ActorSpawnFailed, ActorSpawned, SpawnActor};

use bevy::prelude::*;

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
		.add_message::<ActorSpawnFailed>();
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
