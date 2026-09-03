//! In-game clap commands for developments-on-terrain.

use bevy::prelude::*;
use clap::{Parser, Subcommand};
use game_commands::command::{CommandScript, GameCommand};

pub const PLAYGROUND_CLI_NAME: &str = "richmond-developments-on-terrain";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "richmond-developments-on-terrain",
	version,
	about = "Les Halles developments on Durham terrain (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Set the world generation seed and regenerate.
	Seed {
		value: u32,
	},
	/// Fill likelihood for Les Halles cells (`0…1`).
	Likelihood {
		value: f32,
	},
	/// Fine-grid Chebyshev half-extent in terrain cells.
	TerrainRadius {
		cells: i32,
	},
	/// Rebuild pads and developments without changing the seed.
	Rebuild,
	/// LOD / mesh CPU proxies.
	#[command(subcommand)]
	Stats(Stats),
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Stats {
	Mesh,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestSeed(pub u32);

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestLikelihood(pub f32);

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestTerrainRadius(pub i32);

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestRebuild;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestMeshStats;

impl PlaygroundCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			PlaygroundCommand::Help => *console = Self::long_help_string(),
			PlaygroundCommand::Script(s) => s.run(commands, console),
			PlaygroundCommand::Seed { value } => {
				commands.spawn(RequestSeed(value));
				*console = format!("seed {value}: regenerating");
			}
			PlaygroundCommand::Likelihood { value } => {
				commands.spawn(RequestLikelihood(value));
				*console = format!("likelihood {value}: regenerating");
			}
			PlaygroundCommand::TerrainRadius { cells } => {
				commands.spawn(RequestTerrainRadius(cells.max(1)));
				*console = format!("terrain-radius {}: pending", cells.max(1));
			}
			PlaygroundCommand::Rebuild => {
				commands.spawn(RequestRebuild);
				*console = "rebuild: pending".into();
			}
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
	const CLI_NAME: &str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_seed() {
		let cmd = PlaygroundCommand::parse_line("seed 7").expect("parse");
		assert!(matches!(cmd, PlaygroundCommand::Seed { value: 7 }));
	}

	#[test]
	fn parse_likelihood() {
		let cmd = PlaygroundCommand::parse_line("likelihood 0.5").expect("parse");
		assert!(
			matches!(cmd, PlaygroundCommand::Likelihood { value } if (value - 0.5).abs() < 1e-6)
		);
	}
}
