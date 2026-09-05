//! In-game clap command hierarchy.

use bevy::prelude::*;
use chico_forests::{
	parse_layering_kind, ForestStream, ForestStreamSpec, LayeringKind, DEFAULT_FOREST_NOISE,
	DEFAULT_FOREST_STREAM_RADIUS,
};
use clap::{Parser, Subcommand};
use game_commands::command::{CommandScript, GameCommand};
use procedural_common::{noise_params_from_scalar_str, NoiseParams};

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
	/// Stream the unified Chico forest (Hopscotch 1600 m cells).
	Forest(ForestArgs),
	/// LOD / mesh CPU proxies (triangle counts, etc.).
	#[command(subcommand)]
	Stats(Stats),
}

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct ForestArgs {
	/// Pin a well-known layering (`lush-jungle`, `ag-town`, …). Omit to Hopscotch.
	#[arg(value_parser = parse_layering_kind, value_name = "LAYERING")]
	pub layering: Option<LayeringKind>,

	/// Hopscotch / layer-throw noise (`seed,frequency,amplitude,octaves[,type]`).
	#[arg(
		long,
		default_value = DEFAULT_FOREST_NOISE,
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]"
	)]
	pub noise: NoiseParams,

	/// Present-ring multiplier (`1` = 1 km present / 3 km generate; `0` = one 100 m tile).
	#[arg(long, default_value_t = DEFAULT_FOREST_STREAM_RADIUS)]
	pub stream_radius: u32,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Stats {
	/// Mesh triangle counts plus foliage / stick / structural LOD probe hosts.
	Mesh,
	/// Toggle throttled `[sbs.timing]` FPS / frame_ms logs.
	Fps,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestMeshStats;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestFpsToggle;

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
			PlaygroundCommand::Forest(args) => {
				commands.insert_resource(ForestStream(Some(ForestStreamSpec {
					noise: args.noise,
					stream_radius: args.stream_radius,
					layering: args.layering,
				})));
				*console = "forest: streaming".into();
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
			Stats::Fps => {
				commands.spawn(RequestFpsToggle);
				*console = "stats fps: toggling".into();
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
	fn forest_defaults_parse() {
		let cmd = PlaygroundCommand::parse_line("forest").expect("parse");
		match cmd {
			PlaygroundCommand::Forest(args) => {
				assert!(args.layering.is_none());
				assert_eq!(args.stream_radius, DEFAULT_FOREST_STREAM_RADIUS);
			}
			_ => panic!("expected forest"),
		}
	}
}
