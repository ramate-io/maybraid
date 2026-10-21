//! Actor construction interpreter. SpawnActor creates the body; handoff
//! activates it.

pub mod lifecycle;
pub mod realize;
pub mod request;

pub use lifecycle::ActorSpawnWork;
pub use request::{ActorRecipe, ActorSpawnFailed, ActorSpawned, CancelActorSpawn, SpawnActor};

use bevy::prelude::*;

#[derive(Resource, Default)]
struct ActorSpawnRuntime;

/// Headless-safe actor construction messages and pending work.
pub fn register_actor_spawn_runtime(app: &mut App) {
	if app.world().contains_resource::<ActorSpawnRuntime>() {
		return;
	}
	app.init_resource::<ActorSpawnRuntime>()
		.init_resource::<ActorSpawnWork>()
		.add_message::<SpawnActor>()
		.add_message::<ActorSpawned>()
		.add_message::<ActorSpawnFailed>()
		.add_message::<CancelActorSpawn>()
		.add_systems(Update, cancel_actor_spawns);
}

pub struct ActorSpawnPlugin;

impl Plugin for ActorSpawnPlugin {
	fn build(&self, app: &mut App) {
		register_actor_spawn_runtime(app);
	}
}

fn cancel_actor_spawns(
	mut work: ResMut<ActorSpawnWork>,
	mut cancels: MessageReader<CancelActorSpawn>,
) {
	for cancel in cancels.read() {
		let _ = work.cancel(cancel.identity);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_spawns_core::{SpawnIdentity, SpawnOwner, SpawnRequestId, SpawnSessionId};
	use world_player::{AwaitingTerrainSurface, CharacterLocomotion, OffTerrainAnchor, Player};

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
	}

	fn construct_world() -> World {
		let mut world = World::new();
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<StandardMaterial>>();
		world.init_resource::<CharacterLocomotion>();
		world.init_resource::<ActorSpawnWork>();
		world
	}

	#[test]
	fn register_actor_spawn_runtime_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_actor_spawn_runtime(&mut app);
		register_actor_spawn_runtime(&mut app);
		assert!(app.world().contains_resource::<ActorSpawnWork>());
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
					SpawnActor::new(
						identity(),
						Vec3::new(8.0, 4.0, 5.0),
						false,
						ActorRecipe::species_default(),
					)
					.realize(&mut commands, &mut meshes, &mut materials, locomotion.as_ref())
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
					SpawnActor::new(
						identity(),
						Vec3::new(8.0, 12.0, 5.0),
						true,
						ActorRecipe::species_default(),
					)
					.realize(&mut commands, &mut meshes, &mut materials, locomotion.as_ref())
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
