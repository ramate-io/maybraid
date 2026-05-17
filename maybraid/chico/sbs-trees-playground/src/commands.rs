//! In-game clap command hierarchy and plugins.

pub mod plugin;
pub mod render;

use bevy::prelude::*;
use clap::Parser;
use game_commands::command::{CommandScript, GameCommand};
pub use plugin::PlaygroundCommandsPlugin;
pub use render::Render;

pub const PLAYGROUND_CLI_NAME: &str = "chico-sbs";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "chico-sbs",
	version,
	about = "chico-sbs playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	#[command(subcommand)]
	Render(Render),
}

impl PlaygroundCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn parse_startup_from_argv_tail(
		tail: Vec<std::ffi::OsString>,
	) -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_from_argv_tail(tail)
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			PlaygroundCommand::Help => *console = Self::long_help_string(),
			PlaygroundCommand::Script(s) => s.run(commands, console),
			PlaygroundCommand::Render(r) => r.react(commands),
		}
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		<Self as GameCommand>::parse_line(line)
	}
}

impl GameCommand for PlaygroundCommand {
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}
