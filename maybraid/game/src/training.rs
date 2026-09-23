//! Tear down Training Ground fixtures when the shared world shell returns home.

use bevy::prelude::*;
use maybraid_world::{TrainingFixture, TrainingPlaced, WorldSurfaceReady};
use menu_screens::GameMode;

use crate::flow::PlaySession;

pub(crate) fn reset_surface_ready(mut ready: ResMut<WorldSurfaceReady>) {
	ready.0 = false;
}

/// Leave / Home: drop the wall and the session mobs. Discovery mobs are unmarked.
pub(crate) fn clear_play_session(
	mut session: ResMut<PlaySession>,
	mut mode: ResMut<GameMode>,
	mut ready: ResMut<WorldSurfaceReady>,
	mut commands: Commands,
	fixtures: Query<Entity, With<TrainingFixture>>,
) {
	for entity in &fixtures {
		commands.entity(entity).despawn();
	}
	commands.remove_resource::<TrainingPlaced>();
	ready.0 = false;
	*session = PlaySession::None;
	mode.label = String::from(PlaySession::Discovery.label());
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn leave_drops_the_fixtures_and_the_session() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaySession::Training);
		world.insert_resource(GameMode::new("Training Ground"));
		world.insert_resource(TrainingPlaced);
		world.insert_resource(WorldSurfaceReady(true));
		world.spawn(TrainingFixture);
		world
			.run_system_once(clear_play_session)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<PlaySession>(), PlaySession::None);
		assert_eq!(world.resource::<GameMode>().label, "Discovery");
		assert!(!world.resource::<WorldSurfaceReady>().0);
		assert!(world.get_resource::<TrainingPlaced>().is_none());
		assert_eq!(world.query::<&TrainingFixture>().iter(&world).count(), 0);
		Ok(())
	}
}
