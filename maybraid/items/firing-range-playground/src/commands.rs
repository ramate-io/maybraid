//! In-game clap commands for the firing range.

use bevy::prelude::*;
use clap::{Args, Parser};
use firearms::WeaponsArmed;
use game_commands::command::{CommandScript, GameCommand};

use crate::session::{RangeSession, DEFAULT_FFA_NPCS};

pub const PLAYGROUND_CLI_NAME: &str = "firing-range";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "firing-range",
	version,
	about = "Firing range commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Stop the held firearm from firing.
	Pause,
	/// Resume firing.
	Resume,
	/// One generated player vs n generated NPCs. Combat waits for a player shot.
	FreeForAll(FreeForAllArgs),
	/// Restore the 1v1 pad fight (default clothing, bullpup).
	Duel,
	/// Stationary dummy with no gun. Fire at it to check projectile collisions.
	#[command(visible_alias = "dummy")]
	TestDummy,
}

#[derive(Clone, Args, Debug, Default, PartialEq, Eq)]
#[command(rename_all = "kebab-case")]
pub struct FreeForAllArgs {
	/// How many NPCs to roll. Default 6.
	#[arg(long, default_value_t = DEFAULT_FFA_NPCS)]
	pub npcs: u16,
	/// Optional loadout RNG seed. Omit for entropy.
	#[arg(long)]
	pub seed: Option<u64>,
}

impl PlaygroundCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Help => *console = Self::long_help_string(),
			Self::Script(script) => script.run(commands, console),
			Self::Pause => {
				commands.insert_resource(WeaponsArmed(false));
				*console = "pause".into();
			}
			Self::Resume => {
				commands.insert_resource(WeaponsArmed(true));
				*console = "resume".into();
			}
			Self::FreeForAll(args) => {
				let npcs = args.npcs.max(1);
				commands.queue(move |world: &mut World| {
					let mut session = world.resource_mut::<RangeSession>();
					session.enter_free_for_all(npcs, args.seed);
				});
				*console = match args.seed {
					Some(seed) => format!("free-for-all npcs={npcs} seed={seed}"),
					None => format!("free-for-all npcs={npcs}"),
				};
			}
			Self::Duel => {
				commands.queue(move |world: &mut World| {
					world.resource_mut::<RangeSession>().enter_duel();
				});
				*console = "duel".into();
			}
			Self::TestDummy => {
				commands.queue(move |world: &mut World| {
					world.resource_mut::<RangeSession>().enter_test_dummy();
				});
				*console = "test-dummy".into();
			}
		}
	}
}

impl GameCommand for PlaygroundCommand {
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_pause() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("pause")?;
		assert!(matches!(command, PlaygroundCommand::Pause));
		Ok(())
	}

	#[test]
	fn parses_free_for_all_defaults() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("free-for-all")?;
		assert!(matches!(
			command,
			PlaygroundCommand::FreeForAll(FreeForAllArgs { npcs: DEFAULT_FFA_NPCS, seed: None })
		));
		Ok(())
	}

	#[test]
	fn parses_free_for_all_npcs_and_seed() -> Result<(), String> {
		let command =
			<PlaygroundCommand as GameCommand>::parse_line("free-for-all --npcs 8 --seed 3")?;
		assert!(matches!(
			command,
			PlaygroundCommand::FreeForAll(FreeForAllArgs { npcs: 8, seed: Some(3) })
		));
		Ok(())
	}

	#[test]
	fn parses_test_dummy() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("test-dummy")?;
		assert!(matches!(command, PlaygroundCommand::TestDummy));
		let alias = <PlaygroundCommand as GameCommand>::parse_line("dummy")?;
		assert!(matches!(alias, PlaygroundCommand::TestDummy));
		Ok(())
	}
}
