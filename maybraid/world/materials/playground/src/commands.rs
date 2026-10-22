//! In-game clap commands for the world-materials playground.

use std::path::PathBuf;

use bevy::prelude::*;
use clap::{Parser, Subcommand};
use firearms::{laser_hex_ref, laser_pulse_ref, laser_tail_ref, muzzle_flame_ref};
use game_commands::command::{CommandScript, GameCommand};
use material_ref::MaterialRef;
use world_materials::chain::{default_evidence_dir, MythicalChainState};

use crate::chain_capture::{CaptureSet, VideoCapture};
use crate::chain_preview::{ChainPreview, LightingMode, PendingChainHit, PreviewStyle};
use crate::preview::{PreviewConfig, PreviewHost};
use crate::shape::PreviewShape;
use crate::shoot::ShootConfig;

pub const PLAYGROUND_CLI_NAME: &str = "materials";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "materials",
	version,
	about = "World materials playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Put a named recipe on the current preview hull.
	Show {
		/// `hex`, `pulse`, `tail`, `muzzle-flame`, `doric`, `chain`, `standard`, or any world recipe name.
		recipe: String,
	},
	/// Switch the preview / shot hull. `sphere` and `capsule` are the solid baselines.
	Mesh {
		/// `sphere`, `capsule`, `core`, `shell`, `ring`, `cards`, `blast`, `cylinder`, or `taper`.
		shape: PreviewShape,
	},
	/// Fire repeating copies of the current hull. `shoot stop` cancels.
	Shoot {
		#[command(subcommand)]
		action: Option<ShootAction>,
	},
	/// Mythical chain preview, hits, lighting, and evidence capture.
	Chain {
		#[command(subcommand)]
		action: ChainCommand,
	},
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ShootAction {
	/// Stop the repeating launcher.
	Stop,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ChainCommand {
	/// Authored state on both chains, or `pair` for intact + damaged.
	Show {
		/// `pair`, `intact`, `damaged`, `near-breaking`, or `released`.
		state: String,
	},
	/// Set only the left chain.
	Left { state: String },
	/// Set only the right chain.
	Right { state: String },
	/// Day key+fill or night moonlight.
	Lighting { mode: String },
	/// Sine-bend the placeholder mesh (A04 stand-in).
	Deform { enabled: String },
	/// Animate left-chain durability. `chain sweep stop` cancels.
	Sweep { action: Option<String> },
	/// Per-chain seed. Right uses `seed + 18`.
	Seed { seed: i32 },
	/// Glow intensity on both chains.
	Glow { intensity: f32 },
	/// Write validation PNGs under world-materials evidence (or `dir`) and exit.
	Capture { dir: Option<PathBuf> },
	/// Localized hit on the left (owned) chain. `link` is the vertex-color id.
	Hit { link: u32 },
	/// Real world visual style (Native / Sharp / Retro / Antediluvian).
	Style { mode: String },
	/// Gameplay-distance traveling energy; 30 Hz deterministic frames + stills.
	Video { dir: PathBuf },
	/// Reverse energy travel.
	Reverse,
	/// Freeze the standing wave (`speed = 0`).
	Freeze,
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
			Self::Show { recipe } if is_chain_show(&recipe) => {
				enter_chain_pair(commands, console);
			}
			Self::Show { recipe } => {
				let material = material_for_show(&recipe);
				*console = format!("show {}", preview_label(&material));
				commands.queue(move |world: &mut World| {
					*world.resource_mut::<PreviewHost>() = PreviewHost::Catalog;
					world.resource_mut::<PreviewConfig>().material = material;
				});
			}
			Self::Mesh { shape } => {
				*console = format!("mesh {}", shape.label());
				commands.queue(move |world: &mut World| {
					*world.resource_mut::<PreviewHost>() = PreviewHost::Catalog;
					world.resource_mut::<PreviewConfig>().shape = shape;
				});
			}
			Self::Shoot { action: Some(ShootAction::Stop) } => {
				*console = "shoot stop".into();
				commands.queue(|world: &mut World| {
					world.resource_mut::<ShootConfig>().enabled = false;
				});
			}
			Self::Shoot { action: None } => {
				*console = "shoot".into();
				commands.queue(|world: &mut World| {
					*world.resource_mut::<PreviewHost>() = PreviewHost::Catalog;
					let preview = world.resource::<PreviewConfig>().clone();
					let mut shoot = world.resource_mut::<ShootConfig>();
					shoot.enabled = true;
					shoot.material = preview.material;
					shoot.shape = preview.shape;
					shoot.accumulator = shoot.interval;
				});
			}
			Self::Chain { action } => action.react(commands, console),
		}
	}
}

impl ChainCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Show { state } => match state.as_str() {
				"pair" => enter_chain_pair(commands, console),
				other => match parse_state(other) {
					Some(state) => {
						*console = format!("chain show {}", state.label());
						commands.queue(move |world: &mut World| {
							enter_chain_host(world);
							let mut preview = world.resource_mut::<ChainPreview>();
							preview.set_left_state(state);
							preview.set_right_state(state);
						});
					}
					None => *console = format!("unknown state '{other}'"),
				},
			},
			Self::Left { state } => match parse_state(&state) {
				Some(state) => {
					*console = format!("chain left {}", state.label());
					commands.queue(move |world: &mut World| {
						enter_chain_host(world);
						world.resource_mut::<ChainPreview>().set_left_state(state);
					});
				}
				None => *console = format!("unknown state '{state}'"),
			},
			Self::Right { state } => match parse_state(&state) {
				Some(state) => {
					*console = format!("chain right {}", state.label());
					commands.queue(move |world: &mut World| {
						enter_chain_host(world);
						world.resource_mut::<ChainPreview>().set_right_state(state);
					});
				}
				None => *console = format!("unknown state '{state}'"),
			},
			Self::Lighting { mode } => match LightingMode::from_label(&mode) {
				Some(mode) => {
					*console = format!("chain lighting {}", mode.label());
					commands.queue(move |world: &mut World| {
						enter_chain_host(world);
						world.resource_mut::<ChainPreview>().lighting = mode;
						*world.resource_mut::<LightingMode>() = mode;
					});
				}
				None => *console = format!("unknown lighting '{mode}'"),
			},
			Self::Deform { enabled } => match parse_toggle(&enabled) {
				Some(enabled) => {
					*console = format!("chain deform {}", if enabled { "on" } else { "off" });
					commands.queue(move |world: &mut World| {
						enter_chain_host(world);
						world.resource_mut::<ChainPreview>().deform = enabled;
					});
				}
				None => *console = format!("chain deform expects on|off, got '{enabled}'"),
			},
			Self::Sweep { action } => {
				let stop = action.as_deref() == Some("stop");
				*console = if stop { "chain sweep stop".into() } else { "chain sweep".into() };
				commands.queue(move |world: &mut World| {
					enter_chain_host(world);
					world.resource_mut::<ChainPreview>().sweep = !stop;
				});
			}
			Self::Seed { seed } => {
				*console = format!("chain seed {seed}");
				commands.queue(move |world: &mut World| {
					enter_chain_host(world);
					let mut preview = world.resource_mut::<ChainPreview>();
					preview.seed = seed;
					preview.restamp_glow_and_seed();
				});
			}
			Self::Glow { intensity } => {
				*console = format!("chain glow {intensity}");
				commands.queue(move |world: &mut World| {
					enter_chain_host(world);
					let mut preview = world.resource_mut::<ChainPreview>();
					preview.glow_intensity = intensity.clamp(0.0, 8.0);
					preview.restamp_glow_and_seed();
				});
			}
			Self::Capture { dir } => {
				let dir = dir.unwrap_or_else(default_evidence_dir);
				*console = format!("chain capture {}", dir.display());
				commands.queue(move |world: &mut World| {
					enter_chain_host(world);
					*world.resource_mut::<CaptureSet>() = CaptureSet::start(dir);
				});
			}
			Self::Hit { link } => {
				*console = format!("chain hit {link}");
				commands.queue(move |world: &mut World| {
					enter_chain_host(world);
					*world.resource_mut::<PendingChainHit>() =
						PendingChainHit { link: Some(link), amount: 0.72 };
				});
			}
			Self::Style { mode } => match PreviewStyle::from_label(&mode) {
				Some(style) => {
					*console = format!("chain style {}", style.label());
					commands.queue(move |world: &mut World| {
						enter_chain_host(world);
						world.resource_mut::<ChainPreview>().style = style;
					});
				}
				None => *console = format!("unknown style '{mode}'"),
			},
			Self::Video { dir } => {
				*console = format!("chain video {}", dir.display());
				commands.queue(move |world: &mut World| {
					enter_chain_host(world);
					*world.resource_mut::<VideoCapture>() = VideoCapture::start(dir);
				});
			}
			Self::Reverse => {
				*console = "chain reverse".into();
				commands.queue(|world: &mut World| {
					enter_chain_host(world);
					let mut preview = world.resource_mut::<ChainPreview>();
					preview.flow = preview.flow.reversed();
					preview.restamp_glow_and_seed();
				});
			}
			Self::Freeze => {
				*console = "chain freeze".into();
				commands.queue(|world: &mut World| {
					enter_chain_host(world);
					let mut preview = world.resource_mut::<ChainPreview>();
					preview.flow.speed_mps = 0.0;
					preview.restamp_glow_and_seed();
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

fn enter_chain_pair(commands: &mut Commands, console: &mut String) {
	*console = "chain show pair".into();
	commands.queue(|world: &mut World| {
		enter_chain_host(world);
		let mut preview = world.resource_mut::<ChainPreview>();
		preview.set_left_state(MythicalChainState::Intact);
		preview.set_right_state(MythicalChainState::Damaged);
	});
}

fn enter_chain_host(world: &mut World) {
	*world.resource_mut::<PreviewHost>() = PreviewHost::Chain;
	world.resource_mut::<ShootConfig>().enabled = false;
}

fn is_chain_show(name: &str) -> bool {
	matches!(name, "chain" | "mythical_chain" | "mythical-chain")
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

pub(crate) fn material_for_show(name: &str) -> MaterialRef {
	match name {
		"hex" | "laser_hex" | "laser-hex" => laser_hex_ref(),
		"pulse" | "laser_pulse" | "laser-pulse" => laser_pulse_ref(),
		"tail" | "laser_tail" | "laser-tail" => laser_tail_ref(),
		"muzzle-flame" | "muzzle_flame" => muzzle_flame_ref(),
		"doric" | "doric_column" | "doric-column" | "doric-column-shaft-pom" => {
			building_shaders::doric_column_ref()
		}
		"standard" | "default" => MaterialRef::default(),
		other => MaterialRef::named(other),
	}
}

pub(crate) fn preview_label(material: &MaterialRef) -> String {
	match &material.name {
		material_ref::MaterialId::Default => "standard".into(),
		material_ref::MaterialId::Name(name) => name.clone(),
	}
}
