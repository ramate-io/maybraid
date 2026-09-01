//! In-game clap commands for the items playground.

use bevy::prelude::*;
use clap::Parser;
use firearms::FirearmConcept;
use game_commands::command::{CommandScript, GameCommand};

use crate::preview::PreviewConfig;

pub const PLAYGROUND_CLI_NAME: &str = "items";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "items",
	version,
	about = "Items playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Spawn a firearm concept on the shared receiver rig.
	Show {
		concept: FirearmConcept,
	},
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
			Self::Show { concept } => {
				commands.insert_resource(PreviewConfig { concept });
				*console = format!("show {}", concept.label());
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
	fn parses_show_bullpup() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show bullpup")?;
		let PlaygroundCommand::Show { concept } = command else {
			return Err("expected show".into());
		};
		assert_eq!(concept, FirearmConcept::Bullpup);
		Ok(())
	}
}
