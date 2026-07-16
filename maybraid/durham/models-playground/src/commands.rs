//! In-game clap command hierarchy for the Durham terrain models playground.

use bevy::prelude::*;
use clap::{Parser, Subcommand};
use game_commands::command::{CommandScript, GameCommand};

pub const PLAYGROUND_CLI_NAME: &str = "durham-terrain";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "durham-terrain",
	version,
	about = "Durham terrain models playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Configure the fixed terrain cell request region and regenerate.
	#[command(subcommand)]
	Cells(Cells),
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Cells {
	/// Print the current cell size, origin, and extents.
	Show,
	/// Set cell layout fields (omit a flag to keep the current value) and regenerate.
	Set {
		/// Edge length of each origin cell in world units.
		#[arg(long)]
		size: Option<f32>,
		/// Cell-grid origin X (min corner).
		#[arg(long, allow_hyphen_values = true)]
		origin_x: Option<i32>,
		/// Cell-grid origin Z (min corner).
		#[arg(long, allow_hyphen_values = true)]
		origin_z: Option<i32>,
		/// Number of cells along +X.
		#[arg(long)]
		extent_x: Option<u32>,
		/// Number of cells along +Z.
		#[arg(long)]
		extent_z: Option<u32>,
	},
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestCellShow;

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
			PlaygroundCommand::Cells(cells) => cells.react(commands, console),
		}
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		<Self as GameCommand>::parse_line(line)
	}
}

impl Cells {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Cells::Show => {
				commands.spawn(RequestCellShow);
				*console = "cells show: pending".into();
			}
			Cells::Set { size, origin_x, origin_z, extent_x, extent_z } => {
				commands.spawn(PendingCellLayoutPatch {
					size,
					origin_x,
					origin_z,
					extent_x,
					extent_z,
				});
				*console = "cells set: regenerating".into();
			}
		}
	}
}

/// Partial update applied against the current [`TerrainCellLayout`].
#[derive(Component, Debug, Clone)]
pub struct PendingCellLayoutPatch {
	pub size: Option<f32>,
	pub origin_x: Option<i32>,
	pub origin_z: Option<i32>,
	pub extent_x: Option<u32>,
	pub extent_z: Option<u32>,
}

impl GameCommand for PlaygroundCommand {
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}
