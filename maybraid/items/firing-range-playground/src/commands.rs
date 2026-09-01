//! In-game clap commands for the firing range.

use bevy::prelude::*;
use clap::Parser;
use firearms::WeaponsArmed;
use game_commands::command::{CommandScript, GameCommand};

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
	/// Stop auto-fire (lasers freeze).
	Pause,
	/// Resume auto-fire.
	Resume,
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
}
