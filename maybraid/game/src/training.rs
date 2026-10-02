//! Training Ground session: character mode, seeded rounds, and teardown.
//!
//! A session starts on a fresh [`TrainingRound`]. Each respawn advances the
//! round and loads it in behind the loading screen, where [`TrainingSpawn::next`]
//! becomes the round's character mode. A random trainee's next life stays on
//! the same map; your own character's moves to a new one.

use bevy::prelude::*;
use maybraid_game_mode_training_ground::TrainingRound;
use maybraid_world::{TrainingLifeEnded, WorldPlayerLoadout, WorldSurfaceReady};
use menu_screens::{GameMode, TrainingCharacterChoice, TrainingSpawn};

use crate::flow::{GameFlow, PlaySession};
use crate::shell::ShellRoute;

pub(crate) fn reset_surface_ready(mut ready: ResMut<WorldSurfaceReady>) {
	ready.0 = false;
}

/// Training setup pick: start a session on a fresh seed.
pub(crate) fn start_training_session(
	mut choices: MessageReader<TrainingCharacterChoice>,
	mut route: ShellRoute,
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
	route.enter(GameFlow::LoadingWorld, PlaySession::Training);
}

pub(crate) fn reload_training_round(
	mut ended: MessageReader<TrainingLifeEnded>,
	spawn: Option<Res<TrainingSpawn>>,
	session: Res<PlaySession>,
	mut round: ResMut<TrainingRound>,
	mut route: ShellRoute,
) {
	if ended.read().last().is_none() {
		return;
	}
	*round = next_training_round(spawn.as_deref(), *round);
	route.enter(GameFlow::LoadingWorld, *session);
}

fn next_training_round(spawn: Option<&TrainingSpawn>, round: TrainingRound) -> TrainingRound {
	match spawn.map(|spawn| spawn.next) {
		Some(TrainingCharacterChoice::Random) => round.next_life(),
		_ => round.next(),
	}
}

pub(crate) fn begin_training_round(spawn: Option<ResMut<TrainingSpawn>>) {
	if let Some(mut spawn) = spawn {
		spawn.begin_round();
	}
}

/// The round's trainee when this Training round plays a random character.
pub(crate) fn session_trainee(
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
	round.map(|round| maybraid_world::training_trainee(*round))
}

/// Leave / Home: drop the session.
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
	use maybraid_game_mode_training_ground::TrainingGround;
	use layer_stack::ActiveGenerationMode;

	fn shell_states(world: &mut World) {
		world.insert_resource(NextState::<GameFlow>::Unchanged);
		world.insert_resource(NextState::<ActiveGenerationMode>::Unchanged);
	}

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
		shell_states(&mut world);
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
		assert!(matches!(
			world.resource::<NextState<ActiveGenerationMode>>(),
			NextState::PendingIfNeq(mode) if mode.is::<TrainingGround>()
		));
		Ok(())
	}

	fn ended_life(next: TrainingCharacterChoice) -> anyhow::Result<TrainingRound> {
		let mut world = World::new();
		world.init_resource::<Messages<TrainingLifeEnded>>();
		world.write_message(TrainingLifeEnded);
		shell_states(&mut world);
		world.insert_resource(PlaySession::Training);
		world.insert_resource(TrainingRound::new(3));
		let mut spawn = TrainingSpawn::new(TrainingCharacterChoice::Random);
		spawn.next = next;
		world.insert_resource(spawn);
		world
			.run_system_once(reload_training_round)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(matches!(
			world.resource::<NextState<GameFlow>>(),
			NextState::Pending(GameFlow::LoadingWorld)
		));
		assert!(matches!(
			world.resource::<NextState<ActiveGenerationMode>>(),
			NextState::PendingIfNeq(mode) if mode.is::<TrainingGround>()
		));
		Ok(*world.resource::<TrainingRound>())
	}

	#[test]
	fn a_random_trainee_respawns_on_the_same_map() -> anyhow::Result<()> {
		let round = TrainingRound::new(3);
		let next = ended_life(TrainingCharacterChoice::Random)?;
		assert_eq!(next, round.next_life());
		assert_eq!(next.map(), round.map());
		assert_ne!(
			maybraid_world::training_trainee(next),
			maybraid_world::training_trainee(round)
		);
		Ok(())
	}

	#[test]
	fn your_character_respawns_on_a_new_map() -> anyhow::Result<()> {
		let next = ended_life(TrainingCharacterChoice::Active)?;
		assert_eq!(next, TrainingRound::new(3).next());
		Ok(())
	}

	#[test]
	fn only_random_training_rounds_play_the_trainee() {
		let round = TrainingRound::new(5);
		let random = TrainingSpawn::new(TrainingCharacterChoice::Random);
		let active = TrainingSpawn::new(TrainingCharacterChoice::Active);
		assert_eq!(
			session_trainee(PlaySession::Training, Some(&random), Some(&round)),
			Some(maybraid_world::training_trainee(round))
		);
		assert_eq!(session_trainee(PlaySession::Training, Some(&active), Some(&round)), None);
		assert_eq!(session_trainee(PlaySession::Discovery, Some(&random), Some(&round)), None);
		assert_eq!(session_trainee(PlaySession::Training, None, Some(&round)), None);
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
