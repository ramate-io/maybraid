//! Mount and tear down the Training Ground arena on the shared world shell.

use std::hash::{BuildHasher, Hasher};

use bevy::prelude::*;
use les_halles_arena::{mount_arena, ArenaMount, LesHallesSpawn, TrainingArena};
use maybraid_world::WorldSurfaceReady;
use menu_screens::GameMode;

use crate::flow::PlaySession;

pub(crate) fn spawn_training_arena(
	session: Res<PlaySession>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	if *session != PlaySession::Training {
		return;
	}
	mount_arena(&mut commands, &mut meshes, &mut materials, training_seed());
}

pub(crate) fn reset_surface_ready(mut ready: ResMut<WorldSurfaceReady>) {
	ready.0 = false;
}

/// Leave / Home: drop the arena so a later Discovery enter does not keep Halles at the origin.
pub(crate) fn clear_play_session(
	mut session: ResMut<PlaySession>,
	mut mode: ResMut<GameMode>,
	mut ready: ResMut<WorldSurfaceReady>,
	mut commands: Commands,
	arena: Query<Entity, With<TrainingArena>>,
) {
	for entity in &arena {
		commands.entity(entity).despawn();
	}
	commands.remove_resource::<LesHallesSpawn>();
	commands.remove_resource::<ArenaMount>();
	ready.0 = false;
	*session = PlaySession::None;
	mode.label = String::from(PlaySession::Discovery.label());
}

fn training_seed() -> i32 {
	let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
	hasher.write_u128(
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|elapsed| elapsed.as_nanos())
			.unwrap_or(1),
	);
	let seed = hasher.finish() as i32;
	if seed == 0 {
		1
	} else {
		seed
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn leave_drops_the_arena_and_the_session() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaySession::Training);
		world.insert_resource(GameMode::new("Training Ground"));
		world.insert_resource(ArenaMount { floors: 2, failed: false });
		world.insert_resource(LesHallesSpawn::default());
		world.insert_resource(WorldSurfaceReady(true));
		world.spawn(TrainingArena);
		world
			.run_system_once(clear_play_session)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<PlaySession>(), PlaySession::None);
		assert_eq!(world.resource::<GameMode>().label, "Discovery");
		assert!(!world.resource::<WorldSurfaceReady>().0);
		assert!(world.get_resource::<LesHallesSpawn>().is_none());
		assert!(world.get_resource::<ArenaMount>().is_none());
		assert_eq!(world.query::<&TrainingArena>().iter(&world).count(), 0);
		Ok(())
	}
}
