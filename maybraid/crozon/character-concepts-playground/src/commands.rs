//! In-game clap command hierarchy for the concepts playground.

pub mod braidman;
pub mod brodler;
pub mod dui;
pub mod mygr;
pub mod wumbus;

use bevy::prelude::*;
pub use braidman::Braidman;
pub use brodler::Brodler;
pub use dui::Dui;
pub use mygr::Mygr;
pub use wumbus::Wumbus;
use clap::Parser;
use game_commands::command::{CommandScript, GameCommand};

use crate::skinning::request_dump_bones;

pub const CONCEPTS_CLI_NAME: &str = "crozon-concepts";
pub type Script = CommandScript<ConceptsCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "crozon-concepts",
	version,
	about = "Crozon character concepts playground commands",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum ConceptsCommand {
	Help,
	Script(Script),
	/// Spawn or adjust the simplified Braidman concept preview.
	#[command(subcommand)]
	Braidman(Braidman),
	/// Spawn or adjust the Brodler concept preview.
	#[command(subcommand)]
	Brodler(Brodler),
	/// Spawn or adjust the Mygr concept preview.
	#[command(subcommand)]
	Mygr(Mygr),
	/// Spawn or adjust the Dui concept preview.
	#[command(subcommand)]
	Dui(Dui),
	/// Spawn or adjust the Wumbus concept preview.
	#[command(subcommand)]
	Wumbus(Wumbus),
	/// Print the live rig bone hierarchy to the HUD console.
	DumpBones,
}

impl ConceptsCommand {
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
			Self::Braidman(braidman) => braidman.react(commands),
			Self::Brodler(brodler) => brodler.react(commands),
			Self::Mygr(mygr) => mygr.react(commands),
			Self::Dui(dui) => dui.react(commands),
			Self::Wumbus(wumbus) => wumbus.react(commands),
			Self::DumpBones => request_dump_bones(commands),
		}
	}
}

impl GameCommand for ConceptsCommand {
	const CLI_NAME: &'static str = CONCEPTS_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}
