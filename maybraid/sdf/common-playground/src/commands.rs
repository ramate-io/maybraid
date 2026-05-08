pub mod react;
pub mod render;
pub mod settings;

use bevy::prelude::*;
use clap::{CommandFactory, Parser};
pub use render::Render;
pub use settings::Settings;

/// Root command parsed from in-game text (after `/`); not used for process argv.
///
/// Example lines (typed + Enter):
/// - `help`
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

	/// Spawn this command (and nested subcommands) so [`crate::commands::react`] systems can respond.
	pub fn react(self, commands: &mut Commands) {
		commands.spawn(self.clone());
		match self {
			PlaygroundCommand::Help => {}
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
