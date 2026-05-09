//! In-game clap command hierarchy and plugins.

pub mod plugin;
pub mod render;
pub(crate) mod root;
pub mod script;

use bevy::prelude::*;
use clap::{CommandFactory, Parser};
pub use plugin::PlaygroundCommandsPlugin;
pub use render::Render;
pub use script::Script;

pub const PLAYGROUND_CLI_NAME: &str = "chico-sbs";

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
		Self::command().render_long_help().to_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		let tail: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
		Self::parse_startup_from_argv_tail(tail)
	}

	pub fn parse_startup_from_argv_tail(tail: Vec<std::ffi::OsString>) -> Result<Option<Self>, String> {
		if tail.is_empty() {
			return Ok(None);
		}
		let mut args = vec![std::ffi::OsString::from(PLAYGROUND_CLI_NAME)];
		args.extend(tail);
		Self::try_parse_from(args).map(Some).map_err(|e| e.to_string())
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		if let PlaygroundCommand::Script(s) = &self {
			script::run_script_file(&s.path, commands, console);
			return;
		}
		commands.spawn(self.clone());
		match self {
			PlaygroundCommand::Help => {}
			PlaygroundCommand::Script(_) => {}
			PlaygroundCommand::Render(r) => r.react(commands),
		}
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		let line = line.trim().trim_start_matches('/').trim();
		let mut args: Vec<String> = vec![PLAYGROUND_CLI_NAME.to_string()];
		for w in line.split_whitespace() {
			if !w.is_empty() {
				args.push(w.to_string());
			}
		}
		if args.len() <= 1 {
			return Err("empty command".into());
		}
		Self::try_parse_from(args).map_err(|e| e.to_string())
	}
}
