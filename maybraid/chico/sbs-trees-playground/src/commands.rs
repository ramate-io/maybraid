//! In-game clap command hierarchy.

pub mod render;
pub mod show;

use bevy::prelude::*;
use clap::{Parser, Subcommand};
use game_commands::command::{CommandScript, GameCommand};
pub use render::Render;
pub use show::Show;

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
	/// LodScene / VegetationComponents presentation (migrated trees).
	#[command(subcommand)]
	Show(Show),
	/// LOD / mesh CPU proxies (triangle counts, etc.).
	#[command(subcommand)]
	Stats(Stats),
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Stats {
	/// Sum vertex / index / triangle counts from spawned `Mesh3d` assets.
	Mesh,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestMeshStats;

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
			PlaygroundCommand::Show(s) => s.react(commands),
			PlaygroundCommand::Stats(stats) => stats.react(commands, console),
		}
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		<Self as GameCommand>::parse_line(line)
	}
}

impl Stats {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Stats::Mesh => {
				commands.spawn(RequestMeshStats);
				*console = "stats mesh: pending".into();
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
