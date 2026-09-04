//! In-game clap commands for the routing playground.

use bevy::prelude::*;
use clap::{Parser, Subcommand};
use game_commands::command::{CommandScript, GameCommand};

pub const PLAYGROUND_CLI_NAME: &str = "routing-playground";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "routing-playground",
	version,
	about = "Hierarchical routing on Durham (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Switch between free-look fly camera and third-person character control.
	#[command(subcommand)]
	Mode(Mode),
	/// Set the router's destination in world XZ metres (disables tether grant).
	Go {
		#[arg(allow_hyphen_values = true)]
		x: f32,
		#[arg(allow_hyphen_values = true)]
		z: f32,
	},
	/// Leash the NPC to the player.
	Tether {
		#[arg(default_value_t = 8.0)]
		radius: f32,
	},
	/// Keep the NPC on a ring around the player.
	Stalk {
		#[arg(default_value_t = 12.0)]
		radius: f32,
	},
	/// Higher-order grant off: tether stays installed but does not write.
	Idle,
	/// Higher-order grant on.
	Drive,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Mode {
	/// Free-look fly camera (WASD + mouse, Space/Shift vertical).
	Free,
	/// Capsule with third-person camera (WASD move, Space jump).
	Character,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestModeFree;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestModeCharacter;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestGo {
	pub x: f32,
	pub z: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestTether {
	pub radius: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestStalk {
	pub radius: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestTetherIdle;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestTetherDrive;

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
			PlaygroundCommand::Mode(mode) => mode.react(commands, console),
			PlaygroundCommand::Go { x, z } => {
				commands.spawn(RequestGo { x, z });
				*console = format!("go {x:.0} {z:.0}: pending");
			}
			PlaygroundCommand::Tether { radius } => {
				commands.spawn(RequestTether { radius });
				*console = format!("tether {radius:.0}: pending");
			}
			PlaygroundCommand::Stalk { radius } => {
				commands.spawn(RequestStalk { radius });
				*console = format!("stalk {radius:.0}: pending");
			}
			PlaygroundCommand::Idle => {
				commands.spawn(RequestTetherIdle);
				*console = "idle: pending".into();
			}
			PlaygroundCommand::Drive => {
				commands.spawn(RequestTetherDrive);
				*console = "drive: pending".into();
			}
		}
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		<Self as GameCommand>::parse_line(line)
	}
}

impl Mode {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Mode::Free => {
				commands.spawn(RequestModeFree);
				*console = "mode free: pending".into();
			}
			Mode::Character => {
				commands.spawn(RequestModeCharacter);
				*console = "mode character: pending".into();
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
