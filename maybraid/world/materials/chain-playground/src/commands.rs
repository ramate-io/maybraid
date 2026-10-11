//! In-game clap commands for the mythical-chain playground.

use std::path::PathBuf;

use bevy::prelude::*;
use chain_shaders::MythicalChainState;
use clap::Parser;
use game_commands::command::{CommandScript, GameCommand};

use crate::capture::CaptureSet;
use crate::preview::{LightingMode, PreviewConfig};

pub const PLAYGROUND_CLI_NAME: &str = "chain";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "chain",
	version,
	about = "Mythical chain playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Authored state on both chains, or `pair` for intact + damaged.
	Show {
		/// `pair`, `intact`, `damaged`, `near-breaking`, or `released`.
		state: String,
	},
	/// Set only the left chain.
	Left {
		state: String,
	},
	/// Set only the right chain.
	Right {
		state: String,
	},
	/// Day key+fill or night moonlight.
	Lighting {
		mode: String,
	},
	/// Sine-bend the placeholder mesh (A04 stand-in).
	Deform {
		enabled: String,
	},
	/// Animate left-chain durability. `sweep stop` cancels.
	Sweep {
		action: Option<String>,
	},
	/// Per-chain seed. Right uses `seed + 18`.
	Seed {
		seed: i32,
	},
	/// Glow intensity on both chains.
	Glow {
		intensity: f32,
	},
	/// Write validation PNGs (pair day/night, near-breaking, released, mixed pair) and exit.
	Capture {
		dir: PathBuf,
	},
}

impl PlaygroundCommand {
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
			Self::Show { state } => match state.as_str() {
				"pair" => {
					*console = "show pair".into();
					commands.queue(|world: &mut World| {
						let mut preview = world.resource_mut::<PreviewConfig>();
						preview.set_left_state(MythicalChainState::Intact);
						preview.set_right_state(MythicalChainState::Damaged);
					});
				}
				other => match parse_state(other) {
					Some(state) => {
						*console = format!("show {}", state.label());
						commands.queue(move |world: &mut World| {
							let mut preview = world.resource_mut::<PreviewConfig>();
							preview.set_left_state(state);
							preview.set_right_state(state);
						});
					}
					None => *console = format!("unknown state '{other}'"),
				},
			},
			Self::Left { state } => match parse_state(&state) {
				Some(state) => {
					*console = format!("left {}", state.label());
					commands.queue(move |world: &mut World| {
						world.resource_mut::<PreviewConfig>().set_left_state(state);
					});
				}
				None => *console = format!("unknown state '{state}'"),
			},
			Self::Right { state } => match parse_state(&state) {
				Some(state) => {
					*console = format!("right {}", state.label());
					commands.queue(move |world: &mut World| {
						world.resource_mut::<PreviewConfig>().set_right_state(state);
					});
				}
				None => *console = format!("unknown state '{state}'"),
			},
			Self::Lighting { mode } => match LightingMode::from_label(&mode) {
				Some(mode) => {
					*console = format!("lighting {}", mode.label());
					commands.queue(move |world: &mut World| {
						world.resource_mut::<PreviewConfig>().lighting = mode;
						*world.resource_mut::<LightingMode>() = mode;
					});
				}
				None => *console = format!("unknown lighting '{mode}'"),
			},
			Self::Deform { enabled } => match parse_toggle(&enabled) {
				Some(enabled) => {
					*console = format!("deform {}", if enabled { "on" } else { "off" });
					commands.queue(move |world: &mut World| {
						world.resource_mut::<PreviewConfig>().deform = enabled;
					});
				}
				None => *console = format!("deform expects on|off, got '{enabled}'"),
			},
			Self::Sweep { action } => {
				let stop = action.as_deref() == Some("stop");
				*console = if stop { "sweep stop".into() } else { "sweep".into() };
				commands.queue(move |world: &mut World| {
					world.resource_mut::<PreviewConfig>().sweep = !stop;
				});
			}
			Self::Seed { seed } => {
				*console = format!("seed {seed}");
				commands.queue(move |world: &mut World| {
					let mut preview = world.resource_mut::<PreviewConfig>();
					preview.seed = seed;
					preview.restamp_glow_and_seed();
				});
			}
			Self::Glow { intensity } => {
				*console = format!("glow {intensity}");
				commands.queue(move |world: &mut World| {
					let mut preview = world.resource_mut::<PreviewConfig>();
					preview.glow_intensity = intensity.clamp(0.0, 8.0);
					preview.restamp_glow_and_seed();
				});
			}
			Self::Capture { dir } => {
				*console = format!("capture {}", dir.display());
				commands.queue(move |world: &mut World| {
					*world.resource_mut::<CaptureSet>() = CaptureSet::start(dir);
				});
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

fn parse_state(label: &str) -> Option<MythicalChainState> {
	MythicalChainState::from_label(label)
}

fn parse_toggle(label: &str) -> Option<bool> {
	match label {
		"on" | "true" | "1" => Some(true),
		"off" | "false" | "0" => Some(false),
		_ => None,
	}
}
