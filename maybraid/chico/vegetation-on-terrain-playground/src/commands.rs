//! In-game clap commands for vegetation-on-terrain.

use bevy::prelude::*;
use clap::{Parser, Subcommand, ValueEnum};
use game_commands::command::{CommandScript, GameCommand};

pub const PLAYGROUND_CLI_NAME: &str = "chico-vegetation-on-terrain";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum GroveKind {
	#[default]
	MonsterGrass,
	BraidGrass,
	TropicalTufts,
	CommonTufts,
	TallGrass,
	WildGrass,
	BushScrub,
	TropicalUndergrowth,
	LevantineScrub,
	StrangeOasis,
	TropicalThicket,
	RollingOaks,
	Orchard,
	RiparianGeneral,
	ForlornSavanna,
	GoettingenFollow,
	Vineyard,
	Dryland,
	Leeward,
	TemperateLowerMassives,
	TemperateMassives,
	Storytellers,
	WanderingAcacia,
	TradeWinds,
	HighBush,
	SpottyBushes,
	RiverineGreen,
	LowBush,
	JungleMassives,
	JungleLowerMassives,
	UnendingJungle,
	JerrysChaparral,
	RiparianMix,
	Alpine,
	ChristmasTaiga,
	ConiferSapling,
	AridConiferSapling,
	ConiferMassives,
	PalmShade,
	Shamanhome,
	DateGrove,
}

impl GroveKind {
	pub fn label(self) -> &'static str {
		match self {
			Self::MonsterGrass => "monster-grass",
			Self::BraidGrass => "braid-grass",
			Self::TropicalTufts => "tropical-tufts",
			Self::CommonTufts => "common-tufts",
			Self::TallGrass => "tall-grass",
			Self::WildGrass => "wild-grass",
			Self::BushScrub => "bush-scrub",
			Self::TropicalUndergrowth => "tropical-undergrowth",
			Self::LevantineScrub => "levantine-scrub",
			Self::StrangeOasis => "strange-oasis",
			Self::TropicalThicket => "tropical-thicket",
			Self::RollingOaks => "rolling-oaks",
			Self::Orchard => "orchard",
			Self::RiparianGeneral => "riparian-general",
			Self::ForlornSavanna => "forlorn-savanna",
			Self::GoettingenFollow => "goettingen-follow",
			Self::Vineyard => "vineyard",
			Self::Dryland => "dryland",
			Self::Leeward => "leeward",
			Self::TemperateLowerMassives => "temperate-lower-massives",
			Self::TemperateMassives => "temperate-massives",
			Self::Storytellers => "storytellers",
			Self::WanderingAcacia => "wandering-acacia",
			Self::TradeWinds => "trade-winds",
			Self::HighBush => "high-bush",
			Self::SpottyBushes => "spotty-bushes",
			Self::RiverineGreen => "riverine-green",
			Self::LowBush => "low-bush",
			Self::JungleMassives => "jungle-massives",
			Self::JungleLowerMassives => "jungle-lower-massives",
			Self::UnendingJungle => "unending-jungle",
			Self::JerrysChaparral => "jerrys-chaparral",
			Self::RiparianMix => "riparian-mix",
			Self::Alpine => "alpine",
			Self::ChristmasTaiga => "christmas-taiga",
			Self::ConiferSapling => "conifer-sapling",
			Self::AridConiferSapling => "arid-conifer-sapling",
			Self::ConiferMassives => "conifer-massives",
			Self::PalmShade => "palm-shade",
			Self::Shamanhome => "shamanhome",
			Self::DateGrove => "date-grove",
		}
	}
}

#[derive(Clone, Parser, Component)]
#[command(
	name = "chico-vegetation-on-terrain",
	version,
	about = "Groves on a small Durham patch (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// One grove type across the tiled footprint.
	Grove {
		kind: GroveKind,
	},
	/// Fine-grid Chebyshev half-extent in terrain cells.
	TerrainRadius {
		cells: i32,
	},
	/// Square grove tile edge in metres.
	GroveExtent {
		meters: f32,
	},
	/// Grove-tile Chebyshev half-extent (`[-r, r]` tiles per axis).
	TileRadius {
		tiles: i32,
	},
	/// Despawn and rebuild groves on the current terrain.
	Rebuild,
	/// LOD / mesh CPU proxies (triangle counts, etc.).
	#[command(subcommand)]
	Stats(Stats),
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Stats {
	/// Mesh triangle counts plus foliage / stick / structural LOD probe hosts.
	Mesh,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestGrove(pub GroveKind);

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestTerrainRadius(pub i32);

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestGroveExtent(pub f32);

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestTileRadius(pub i32);

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
			PlaygroundCommand::Grove { kind } => {
				commands.spawn(RequestGrove(kind));
				*console = format!("grove {}: pending", kind.label());
			}
			PlaygroundCommand::TerrainRadius { cells } => {
				commands.spawn(RequestTerrainRadius(cells.max(1)));
				*console = format!("terrain-radius {}: pending", cells.max(1));
			}
			PlaygroundCommand::GroveExtent { meters } => {
				let meters = meters.max(1.0);
				commands.spawn(RequestGroveExtent(meters));
				*console = format!("grove-extent {meters}: pending");
			}
			PlaygroundCommand::TileRadius { tiles } => {
				commands.spawn(RequestTileRadius(tiles.max(0)));
				*console = format!("tile-radius {}: pending", tiles.max(0));
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
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_grove_and_radii() {
		let grove = PlaygroundCommand::parse_line("grove rolling-oaks").unwrap();
		assert!(matches!(grove, PlaygroundCommand::Grove { kind: GroveKind::RollingOaks }));
		let terrain = PlaygroundCommand::parse_line("terrain-radius 3").unwrap();
		assert!(matches!(terrain, PlaygroundCommand::TerrainRadius { cells: 3 }));
		let tiles = PlaygroundCommand::parse_line("tile-radius 0").unwrap();
		assert!(matches!(tiles, PlaygroundCommand::TileRadius { tiles: 0 }));
	}

	#[test]
	fn parse_stats_mesh() {
		let cmd = PlaygroundCommand::parse_line("stats mesh").unwrap();
		assert!(matches!(cmd, PlaygroundCommand::Stats(Stats::Mesh)));
	}
}
