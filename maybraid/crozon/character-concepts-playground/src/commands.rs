//! In-game clap command hierarchy for the concepts playground.

pub mod braidman;
pub mod brenal;
pub mod claber;
pub mod croconot;
pub mod brodler;
pub mod dui;
pub mod lero;
pub mod mygr;
pub mod spibmom;
pub mod wumbus;

use bevy::prelude::*;
pub use braidman::Braidman;
pub use brenal::Brenal;
pub use claber::Claber;
pub use croconot::Croconot;
pub use brodler::Brodler;
use clap::Parser;
pub use dui::Dui;
use game_commands::command::{CommandScript, GameCommand};
pub use lero::Lero;
pub use mygr::Mygr;
pub use spibmom::Spibmom;
pub use wumbus::Wumbus;

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
	/// Spawn or adjust the Brenal quadruped concept preview.
	#[command(subcommand)]
	Brenal(Brenal),
	/// Spawn or adjust the Claber oversized low-slung quadruped concept preview.
	#[command(subcommand)]
	Claber(Claber),
	/// Spawn or adjust the Croconot low-slung quadruped concept preview.
	#[command(subcommand)]
	Croconot(Croconot),
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
	/// Spawn or adjust the Lero concept preview.
	#[command(subcommand)]
	Lero(Lero),
	/// Spawn or adjust the Spibmom concept preview.
	#[command(subcommand)]
	Spibmom(Spibmom),
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
			Self::Brenal(brenal) => brenal.react(commands),
			Self::Claber(claber) => claber.react(commands),
			Self::Croconot(croconot) => croconot.react(commands),
			Self::Brodler(brodler) => brodler.react(commands),
			Self::Mygr(mygr) => mygr.react(commands),
			Self::Dui(dui) => dui.react(commands),
			Self::Wumbus(wumbus) => wumbus.react(commands),
			Self::Lero(lero) => lero.react(commands),
			Self::Spibmom(spibmom) => spibmom.react(commands),
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
