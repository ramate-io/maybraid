//! Training Ground session: character mode, seeded rounds, and teardown.
//!
//! A session starts on a fresh [`TrainingRound`]. Each respawn advances the
//! round and loads it in behind the loading screen, where [`TrainingSpawn::next`]
//! becomes the round's character mode.

use bevy::prelude::*;
use maybraid_world::{TrainingRound, TrainingRoundAdvanced, WorldPlayerLoadout, WorldSurfaceReady};
use menu_screens::{GameMode, TrainingCharacterChoice, TrainingSpawn};

use crate::flow::{GameFlow, PlaySession};

pub(crate) fn reset_surface_ready(mut ready: ResMut<WorldSurfaceReady>) {
	ready.0 = false;
}

/// Training setup pick: start a session on a fresh seed.
pub(crate) fn start_training_session(
	mut choices: MessageReader<TrainingCharacterChoice>,
	mut flow: ResMut<NextState<GameFlow>>,
	mut mode: ResMut<GameMode>,
	mut commands: Commands,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	commands.insert_resource(PlaySession::Training);
	commands.insert_resource(TrainingSpawn::new(choice));
	commands.insert_resource(TrainingRound::from_entropy());
	mode.label = String::from(PlaySession::Training.label());
	flow.set(GameFlow::LoadingWorld);
}

pub(crate) fn reload_training_round(
	mut advanced: MessageReader<TrainingRoundAdvanced>,
	mut flow: ResMut<NextState<GameFlow>>,
) {
	if advanced.read().last().is_some() {
		flow.set(GameFlow::LoadingWorld);
	}
}

pub(crate) fn begin_training_round(spawn: Option<ResMut<TrainingSpawn>>) {
	if let Some(mut spawn) = spawn {
		spawn.begin_round();
	}
}

/// The round's trainee when this Training round plays a random character.
pub(crate) fn training_trainee(
	session: PlaySession,
	spawn: Option<&TrainingSpawn>,
	round: Option<&TrainingRound>,
) -> Option<WorldPlayerLoadout> {
	if session != PlaySession::Training {
		return None;
	}
	if spawn?.current != TrainingCharacterChoice::Random {
		return None;
	}
	round.map(|round| round.trainee())
}

/// Leave / Home: drop the session. [`crate::shell`] then clears
/// [`maybraid_game_mode_training_ground::TrainingGroundActive`] so the world
/// fill restores the playable rings.
pub(crate) fn clear_play_session(
	mut commands: Commands,
	mut session: ResMut<PlaySession>,
	mut mode: ResMut<GameMode>,
	mut ready: ResMut<WorldSurfaceReady>,
) {
	ready.0 = false;
	*session = PlaySession::None;
	mode.label = String::from(PlaySession::Discovery.label());
	commands.remove_resource::<TrainingSpawn>();
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
		world.insert_resource(TrainingSpawn::new(TrainingCharacterChoice::Random));
		world
			.run_system_once(clear_play_session)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<PlaySession>(), PlaySession::None);
		assert_eq!(world.resource::<GameMode>().label, "Discovery");
		assert!(!world.resource::<WorldSurfaceReady>().0);
		assert!(world.get_resource::<TrainingSpawn>().is_none());
		Ok(())
	}

	#[test]
	fn a_setup_pick_starts_the_session_on_a_fresh_round() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<TrainingCharacterChoice>>();
		world.write_message(TrainingCharacterChoice::Random);
		world.insert_resource(NextState::<GameFlow>::Unchanged);
		world.insert_resource(GameMode::default());
		world.insert_resource(PlaySession::None);
		world
			.run_system_once(start_training_session)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<PlaySession>(), PlaySession::Training);
		assert_eq!(world.resource::<GameMode>().label, "Training Ground");
		assert_eq!(
			*world.resource::<TrainingSpawn>(),
			TrainingSpawn::new(TrainingCharacterChoice::Random)
		);
		assert!(world.get_resource::<TrainingRound>().is_some());
		assert!(matches!(
			world.resource::<NextState<GameFlow>>(),
			NextState::Pending(GameFlow::LoadingWorld)
		));
		Ok(())
	}

	#[test]
	fn an_advanced_round_loads_in_again() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<TrainingRoundAdvanced>>();
		world.write_message(TrainingRoundAdvanced(TrainingRound::new(3)));
		world.insert_resource(NextState::<GameFlow>::Unchanged);
		world
			.run_system_once(reload_training_round)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(matches!(
			world.resource::<NextState<GameFlow>>(),
			NextState::Pending(GameFlow::LoadingWorld)
		));
		Ok(())
	}

	#[test]
	fn only_random_training_rounds_play_the_trainee() {
		let round = TrainingRound::new(5);
		let random = TrainingSpawn::new(TrainingCharacterChoice::Random);
		let active = TrainingSpawn::new(TrainingCharacterChoice::Active);
		assert_eq!(
			training_trainee(PlaySession::Training, Some(&random), Some(&round)),
			Some(round.trainee())
		);
		assert_eq!(training_trainee(PlaySession::Training, Some(&active), Some(&round)), None);
		assert_eq!(training_trainee(PlaySession::Discovery, Some(&random), Some(&round)), None);
		assert_eq!(training_trainee(PlaySession::Training, None, Some(&round)), None);
	}

	#[test]
	fn the_next_mode_takes_over_when_a_round_begins() -> anyhow::Result<()> {
		let mut world = World::new();
		let mut spawn = TrainingSpawn::new(TrainingCharacterChoice::Active);
		spawn.next = TrainingCharacterChoice::Random;
		world.insert_resource(spawn);
		world
			.run_system_once(begin_training_round)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.resource::<TrainingSpawn>().current, TrainingCharacterChoice::Random);
		Ok(())
	}
}
