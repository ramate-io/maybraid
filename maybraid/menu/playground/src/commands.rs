//! In-game clap command hierarchy.

pub mod loading;
pub mod show;

use bevy::prelude::*;
use clap::Parser;
use game_commands::command::{CommandScript, GameCommand};
pub use loading::Loading;
pub use show::Show;

pub const PLAYGROUND_CLI_NAME: &str = "menu";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "menu",
	version,
	about = "Maybraid menu playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	#[command(subcommand)]
	Show(Show),
	#[command(subcommand)]
	Loading(Loading),
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
			PlaygroundCommand::Show(show) => {
				let label = match &show {
					Show::Home => "home".to_string(),
					Show::Loading => "loading".to_string(),
					Show::Character => "character".to_string(),
					Show::CreateCharacter => "create-character".to_string(),
					Show::InGame { mode } => format!("in-game ({mode})"),
				};
				show.react(commands);
				*console = format!("show {label}: pending");
			}
			PlaygroundCommand::Loading(loading) => loading.react(commands, console),
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

#[cfg(test)]
mod tests {
	use super::{PlaygroundCommand, Show};

	#[test]
	fn parse_line_show_home() {
		let cmd = PlaygroundCommand::parse_line("show home").expect("parse");
		assert!(matches!(cmd, PlaygroundCommand::Show(Show::Home)));
	}

	#[test]
	fn parse_line_show_loading() {
		let cmd = PlaygroundCommand::parse_line("show loading").expect("parse");
		assert!(matches!(cmd, PlaygroundCommand::Show(Show::Loading)));
	}

	#[test]
	fn parse_line_show_character() {
		let cmd = PlaygroundCommand::parse_line("show character").expect("parse");
		assert!(matches!(cmd, PlaygroundCommand::Show(Show::Character)));
	}

	#[test]
	fn parse_line_show_create_character() {
		let cmd = PlaygroundCommand::parse_line("show create-character").expect("parse");
		assert!(matches!(cmd, PlaygroundCommand::Show(Show::CreateCharacter)));
	}

	#[test]
	fn parse_line_show_in_game() {
		let cmd = PlaygroundCommand::parse_line("show in-game").expect("parse");
		assert!(matches!(
			cmd,
			PlaygroundCommand::Show(Show::InGame { ref mode }) if mode == "Discovery"
		));
	}

	#[test]
	fn parse_line_show_in_game_mode() {
		let cmd = PlaygroundCommand::parse_line("show in-game Reliquary").expect("parse");
		assert!(matches!(
			cmd,
			PlaygroundCommand::Show(Show::InGame { ref mode }) if mode == "Reliquary"
		));
	}

	#[test]
	fn parse_line_loading_progress() {
		let cmd = PlaygroundCommand::parse_line("loading progress 0.4").expect("parse");
		assert!(matches!(
			cmd,
			PlaygroundCommand::Loading(super::Loading::Progress { value }) if (value - 0.4).abs() < f32::EPSILON
		));
	}

	#[test]
	fn parse_startup_empty_tail() {
		let none = PlaygroundCommand::parse_startup_from_argv_tail(vec![]).expect("parse");
		assert!(none.is_none());
	}
}
