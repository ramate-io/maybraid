//! In-game clap commands for developments-on-terrain.

use std::ffi::OsString;

use bevy::prelude::*;
use clap::{Parser, Subcommand, ValueEnum};
use game_commands::command::{CommandScript, GameCommand};
use richmond_development_models::DevelopmentConfig;

pub const PLAYGROUND_CLI_NAME: &str = "richmond-developments-on-terrain";
pub type Script = CommandScript<PlaygroundCommand>;

/// Development distribution preset for startup and in-game focus commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum DevelopmentFocus {
	All,
	LesHalles,
	ShepherdsVillage,
	ShepherdsCommune,
	RingFort,
	TempleComplex,
	SingleHighrise,
	SuburbanHomes,
	WizardsTower,
	SkybridgeBazaar,
	OldCityMarket,
}

impl DevelopmentFocus {
	pub fn apply(self, config: &mut DevelopmentConfig) {
		let selected = if self == Self::All { None } else { Some(self) };
		config.les_halles_weight = weight(selected, Self::LesHalles);
		config.shepherds_village_weight = weight(selected, Self::ShepherdsVillage);
		config.shepherds_commune_weight = weight(selected, Self::ShepherdsCommune);
		config.ring_fort_weight = weight(selected, Self::RingFort);
		config.temple_complex_weight = weight(selected, Self::TempleComplex);
		config.single_highrise_weight = weight(selected, Self::SingleHighrise);
		config.suburban_homes_weight = weight(selected, Self::SuburbanHomes);
		config.wizards_tower_weight = weight(selected, Self::WizardsTower);
		config.skybridge_bazaar_weight = weight(selected, Self::SkybridgeBazaar);
		config.old_city_market_weight = weight(selected, Self::OldCityMarket);
	}
}

fn weight(selected: Option<DevelopmentFocus>, kind: DevelopmentFocus) -> f32 {
	match selected {
		None => 1.0,
		Some(selected) if selected == kind => 1.0,
		Some(_) => 0.0,
	}
}

/// Startup-only options plus the existing optional startup command.
#[derive(Clone)]
pub struct PlaygroundStartup {
	pub focus_development: Option<DevelopmentFocus>,
	pub command: Option<PlaygroundCommand>,
}

#[derive(Clone, Parser, Component)]
#[command(
	name = "richmond-developments-on-terrain",
	version,
	about = "Richmond developments on Durham terrain (in-game after `/` or process argv)",
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
	/// Fill likelihood for development cells (`0…1`).
	Likelihood {
		value: f32,
	},
	/// Fine-grid Chebyshev half-extent in terrain cells.
	TerrainRadius {
		cells: i32,
	},
	/// Give one development type all distribution weight (`all` restores defaults).
	FocusDevelopment {
		#[arg(value_enum)]
		development: DevelopmentFocus,
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
pub struct RequestDevelopmentFocus(pub DevelopmentFocus);

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

	pub fn parse_startup() -> Result<PlaygroundStartup, String> {
		Self::parse_startup_tail(std::env::args_os().skip(1).collect())
	}

	pub fn parse_startup_tail(mut tail: Vec<OsString>) -> Result<PlaygroundStartup, String> {
		let focus_development = take_focus_development(&mut tail)?;
		let command = <Self as GameCommand>::parse_startup_from_argv_tail(tail)?;
		Ok(PlaygroundStartup { focus_development, command })
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
			PlaygroundCommand::FocusDevelopment { development } => {
				commands.spawn(RequestDevelopmentFocus(development));
				*console = format!("focus-development {development}: regenerating");
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

impl std::fmt::Display for DevelopmentFocus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let value = self
			.to_possible_value()
			.map(|value| value.get_name().to_owned())
			.unwrap_or_else(|| "unknown".to_owned());
		f.write_str(&value)
	}
}

fn take_focus_development(tail: &mut Vec<OsString>) -> Result<Option<DevelopmentFocus>, String> {
	let mut found = None;
	let mut index = 0;
	while index < tail.len() {
		let value = tail[index].to_string_lossy();
		let inline = value.strip_prefix("--focus-development=");
		if value == "--focus-development" || inline.is_some() {
			if found.is_some() {
				return Err("--focus-development may only be specified once".into());
			}
			let raw = match inline {
				Some(value) => {
					let value = value.to_owned();
					tail.remove(index);
					value
				}
				None => {
					if index + 1 >= tail.len() {
						return Err("--focus-development requires a development name".into());
					}
					tail.remove(index);
					tail.remove(index).to_string_lossy().into_owned()
				}
			};
			found = Some(
				DevelopmentFocus::from_str(&raw, true)
					.map_err(|_| format!("unknown development focus `{raw}`"))?,
			);
			continue;
		}
		index += 1;
	}
	Ok(found)
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
	fn parse_seed() -> Result<(), String> {
		let cmd = PlaygroundCommand::parse_line("seed 7")?;
		assert!(matches!(cmd, PlaygroundCommand::Seed { value: 7 }));
		Ok(())
	}

	#[test]
	fn parse_likelihood() -> Result<(), String> {
		let cmd = PlaygroundCommand::parse_line("likelihood 0.5")?;
		assert!(
			matches!(cmd, PlaygroundCommand::Likelihood { value } if (value - 0.5).abs() < 1e-6)
		);
		Ok(())
	}

	#[test]
	fn startup_focus_flag_preserves_startup_command() -> Result<(), String> {
		let startup = PlaygroundCommand::parse_startup_tail(vec![
			"--focus-development".into(),
			"old-city-market".into(),
			"seed".into(),
			"7".into(),
		])?;
		assert_eq!(startup.focus_development, Some(DevelopmentFocus::OldCityMarket));
		assert!(matches!(startup.command, Some(PlaygroundCommand::Seed { value: 7 })));
		Ok(())
	}

	#[test]
	fn focus_assigns_exclusive_weight() {
		let mut config = DevelopmentConfig::default();
		DevelopmentFocus::TempleComplex.apply(&mut config);
		assert_eq!(config.temple_complex_weight, 1.0);
		assert_eq!(config.old_city_market_weight, 0.0);
		assert_eq!(config.les_halles_weight, 0.0);
	}
}
