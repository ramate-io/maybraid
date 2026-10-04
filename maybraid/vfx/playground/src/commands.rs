//! In-game clap commands for the VFX playground.

use bevy::prelude::*;
use clap::Parser;
use game_commands::command::{CommandScript, GameCommand};
use maybraid_vfx::canonicalize_effect_name;

pub const PLAYGROUND_CLI_NAME: &str = "vfx";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "vfx",
	version,
	about = "VFX playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Spawn a named definition at the origin. Repeat to overlay instances.
	Show {
		/// `firey-explosion`, `flash`, `fireball`, `smoke`, or `sparks`.
		#[arg(default_value = "firey-explosion")]
		effect: String,
		/// Spatial scale (positions, sizes, velocities, light range). Duration is unchanged.
		#[arg(long, default_value_t = 1.0)]
		scale: f32,
		/// Particle-count and flash-intensity scale, clamped by the crate.
		#[arg(long, default_value_t = 1.0)]
		intensity: f32,
	},
}

/// Queued `/show` requests. Applied once [`maybraid_vfx::VfxLibrary`] exists.
#[derive(Resource, Default)]
pub struct PendingVfxShows(pub Vec<VfxShowRequest>);

#[derive(Clone, Debug)]
pub struct VfxShowRequest {
	pub effect: String,
	pub scale: f32,
	pub intensity: f32,
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
			Self::Show { effect, scale, intensity } => {
				let Some(name) = canonicalize_effect_name(&effect) else {
					*console = format!(
						"unknown effect `{effect}` — try firey-explosion, flash, fireball, smoke, sparks"
					);
					return;
				};
				*console = format!("show {name} --scale {scale} --intensity {intensity}");
				let name = name.to_string();
				commands.queue(move |world: &mut World| {
					world.resource_mut::<PendingVfxShows>().0.push(VfxShowRequest {
						effect: name,
						scale,
						intensity,
					});
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
