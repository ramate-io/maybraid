//! Tear down Training Ground fixtures when the shared world shell returns home.

use bevy::prelude::*;
use maybraid_world::WorldSurfaceReady;
use menu_screens::GameMode;

use crate::flow::PlaySession;

pub(crate) fn reset_surface_ready(mut ready: ResMut<WorldSurfaceReady>) {
	ready.0 = false;
}

/// Leave / Home: drop the session. [`crate::shell`] then clears
/// [`maybraid_game_mode_training_ground::TrainingGroundActive`] so the world
/// fill restores the playable rings.
pub(crate) fn clear_play_session(
	mut session: ResMut<PlaySession>,
	mut mode: ResMut<GameMode>,
	mut ready: ResMut<WorldSurfaceReady>,
) {
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
		world.insert_resource(WorldSurfaceReady(true));
		world
			.run_system_once(clear_play_session)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<PlaySession>(), PlaySession::None);
		assert_eq!(world.resource::<GameMode>().label, "Discovery");
		assert!(!world.resource::<WorldSurfaceReady>().0);
		Ok(())
	}
}
