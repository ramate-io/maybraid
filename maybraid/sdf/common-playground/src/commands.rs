//! In-game clap command hierarchy and plugins. Organization: see [`commands/README.md`](commands/README.md).

pub mod plugin;
pub mod render;
pub(crate) mod root;
pub mod script;
pub mod settings;

use bevy::prelude::*;
use clap::{CommandFactory, Parser};
pub use plugin::PlaygroundCommandsPlugin;
pub use render::Render;
pub use script::Script;
pub use settings::Settings;

/// Root command: in-game after `/` ([`parse_line`]) or process argv ([`parse_startup_command`]).
///
/// Examples (typed + Enter, or same tokens after `cargo run -p sdf-common-playground --`):
/// - `help`
/// - `script --path ./setup.txt`
/// - `render tapered-cylinder --res-2 5`
/// - `render noisy-cylinder --noise-amplitude 0.08`
/// - `settings checker-size --meters 5`
/// - `settings seed --value 42`
#[derive(Debug, Clone, Parser, Component)]
#[command(
	name = "sdf",
	version,
	about = "In-game sdf-common playground commands",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	/// Print command reference (shown in the HUD console).
	Help,
	/// Run commands from a file (`--path`): one line per command, same text as after `/`.
	Script(Script),
	/// Update the preview mesh from SDF parameters.
	#[command(subcommand)]
	Render(Render),
	/// Tune playground options (checker scale, seed, …).
	#[command(subcommand)]
	Settings(Settings),
}

impl PlaygroundCommand {
	/// Full `--help`-style text for the HUD.
	pub fn long_help_string() -> String {
		Self::command().render_long_help().to_string()
	}

	/// Parse `std::env::args_os()` after the program name, using the same clap tree as [`parse_line`].
	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		let tail: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
		Self::parse_startup_from_argv_tail(tail)
	}

	/// Same as startup parsing after the binary: `tail` is every `OsString` that follows the program name.
	pub fn parse_startup_from_argv_tail(tail: Vec<std::ffi::OsString>) -> Result<Option<Self>, String> {
		if tail.is_empty() {
			return Ok(None);
		}
		let mut args = vec![std::ffi::OsString::from("sdf")];
		args.extend(tail);
		Self::try_parse_from(args).map(Some).map_err(|e| e.to_string())
	}

	/// Spawn this command and nested subcommands (see [`README.md`](README.md)).
	pub fn react(self, commands: &mut Commands) {
		let mut console = None;
		self.dispatch_react(commands, &mut console);
	}

	/// Like [`react`], but script parse errors and IO errors are written to the HUD console string.
	pub fn react_with_console(self, commands: &mut Commands, console: &mut String) {
		let mut holder = Some(console);
		self.dispatch_react(commands, &mut holder);
	}

	/// Shared by in-game input, startup, and [`script::run_script_file`](script::run_script_file).
	pub(crate) fn dispatch_react(
		self,
		commands: &mut Commands,
		console: &mut Option<&mut String>,
	) {
		if let PlaygroundCommand::Script(s) = &self {
			script::run_script_file(&s.path, commands, console);
			return;
		}
		commands.spawn(self.clone());
		match self {
			PlaygroundCommand::Help => {}
			PlaygroundCommand::Script(_) => {}
			PlaygroundCommand::Render(r) => r.react(commands),
			PlaygroundCommand::Settings(s) => s.react(commands),
		}
	}

	/// Parse a line from text mode: split on whitespace, no shell quoting yet.
	/// A leading `/` (from the toggle key) is ignored.
	pub fn parse_line(line: &str) -> Result<Self, String> {
		let line = line.trim().trim_start_matches('/').trim();
		let mut args: Vec<String> = vec!["sdf".to_string()];
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

#[cfg(test)]
mod tests {
	use std::ffi::OsString;

	use super::PlaygroundCommand;

	#[test]
	fn parse_startup_tail_render() -> anyhow::Result<()> {
		let cmd = PlaygroundCommand::parse_startup_from_argv_tail(vec![
			OsString::from("render"),
			OsString::from("tapered-cylinder"),
			OsString::from("--res-2"),
			OsString::from("5"),
		])
		.map_err(|e| anyhow::anyhow!("{e}"))?
		.ok_or_else(|| anyhow::anyhow!("expected command"))?;
		assert!(matches!(cmd, PlaygroundCommand::Render(_)));
		Ok(())
	}

	#[test]
	fn parse_line_script_matches_startup() -> anyhow::Result<()> {
		let a = PlaygroundCommand::parse_line("script --path ./foo.txt").map_err(|e| anyhow::anyhow!("{e}"))?;
		let b = PlaygroundCommand::parse_startup_from_argv_tail(vec![
			OsString::from("script"),
			OsString::from("--path"),
			OsString::from("./foo.txt"),
		])
		.map_err(|e| anyhow::anyhow!("{e}"))?
		.ok_or_else(|| anyhow::anyhow!("expected command"))?;
		assert!(matches!(a, PlaygroundCommand::Script(_)));
		assert!(matches!(b, PlaygroundCommand::Script(_)));
		Ok(())
	}

	#[test]
	fn parse_startup_empty_tail() -> anyhow::Result<()> {
		let o = PlaygroundCommand::parse_startup_from_argv_tail(vec![]).map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(o.is_none());
		Ok(())
	}
}
