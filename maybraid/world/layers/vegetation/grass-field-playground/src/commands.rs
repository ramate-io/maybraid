//! In-game clap commands for the grass field playground.

use bevy::prelude::*;
use clap::{Parser, Subcommand, ValueEnum};
use game_commands::command::{CommandScript, GameCommand};
use vegetation_components::CanopyShadowMode;
use vegetation_grass_field::{GrassFieldSettings, GrassPatch, GrassPatchLod};

use crate::camera::{yaw_pitch, CameraController};
use crate::ground::TurfPlane;
use crate::scene::{self, PatchKind};

pub const PLAYGROUND_CLI_NAME: &str = "grass-field";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "grass-field",
	version,
	about = "GPU grass field playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	#[command(subcommand)]
	Spawn(SpawnCommand),
	#[command(subcommand)]
	Camera(CameraCommand),
	#[command(subcommand)]
	Wind(WindCommand),
	#[command(subcommand)]
	Lod(LodCommand),
	#[command(subcommand)]
	Shadows(ShadowCommand),
	#[command(subcommand)]
	Compare(CompareCommand),
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum SpawnCommand {
	Default,
	Clump,
	Meadow,
	Slope,
	Crest,
	Clear,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum CameraCommand {
	Orbit,
	Overhead,
	Ground,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WindCommand {
	Set {
		#[arg(long)]
		strength: Option<f32>,
		#[arg(long)]
		speed: Option<f32>,
	},
	Still,
	Gentle,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum LodCommand {
	Host { level: HostLod },
}

#[derive(Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum HostLod {
	High,
	Medium,
	Low,
	UltraLow,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ShadowCommand {
	Proxy,
	Full,
	Off,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum CompareCommand {
	Turf { visible: VisibilityChoice },
	Grass { visible: VisibilityChoice },
}

#[derive(Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum VisibilityChoice {
	On,
	Off,
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
			Self::Spawn(command) => command.react(commands, console),
			Self::Camera(command) => command.react(commands, console),
			Self::Wind(command) => command.react(commands, console),
			Self::Lod(command) => command.react(commands, console),
			Self::Shadows(command) => command.react(commands, console),
			Self::Compare(command) => command.react(commands, console),
		}
	}
}

impl SpawnCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Default => {
				commands.queue(|world: &mut World| {
					despawn_patches(world);
					scene::spawn_kind(world, PatchKind::Meadow);
					scene::spawn_kind(world, PatchKind::Clump);
					scene::spawn_kind(world, PatchKind::Slope);
					scene::spawn_kind(world, PatchKind::Crest);
				});
				*console = "spawned default meadow + clump + slope + crest".into();
			}
			Self::Clear => {
				commands.queue(|world: &mut World| despawn_patches(world));
				*console = "cleared grass patches".into();
			}
			other => {
				let kind = match other {
					Self::Clump => PatchKind::Clump,
					Self::Meadow => PatchKind::Meadow,
					Self::Slope => PatchKind::Slope,
					Self::Crest => PatchKind::Crest,
					Self::Default | Self::Clear => unreachable!(),
				};
				commands.queue(move |world: &mut World| {
					scene::spawn_kind(world, kind);
				});
				*console = format!("spawned {}", kind.label());
			}
		}
	}
}

impl CameraCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		let (from, look) = match self {
			Self::Orbit => (Vec3::new(10.0, 3.2, 16.0), Vec3::new(8.0, 0.3, 8.0)),
			Self::Overhead => (Vec3::new(12.0, 22.0, 12.0), Vec3::new(12.0, 0.0, 12.0)),
			Self::Ground => (Vec3::new(6.0, 0.55, 7.5), Vec3::new(10.0, 0.35, 12.0)),
		};
		commands.queue(move |world: &mut World| {
			let mut cameras =
				world.query_filtered::<(&mut Transform, &mut CameraController), With<Camera3d>>();
			if let Ok((mut transform, mut controller)) = cameras.single_mut(world) {
				*transform = Transform::from_translation(from).looking_at(look, Vec3::Y);
				let (yaw, pitch) = yaw_pitch(transform.rotation);
				controller.yaw = yaw;
				controller.pitch = pitch;
			}
		});
		*console = match self {
			Self::Orbit => "camera orbit".into(),
			Self::Overhead => "camera overhead".into(),
			Self::Ground => "camera ground".into(),
		};
	}
}

impl WindCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Still => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<GrassFieldSettings>().wind.strength_m = 0.0;
				});
				*console = "wind still".into();
			}
			Self::Gentle => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<GrassFieldSettings>().wind =
						vegetation_shaders::wesl::WindInputs::gentle_botw(0.0);
				});
				*console = "wind gentle BotW".into();
			}
			Self::Set { strength, speed } => {
				commands.queue(move |world: &mut World| {
					let mut settings = world.resource_mut::<GrassFieldSettings>();
					if let Some(strength) = strength {
						settings.wind.strength_m = strength.max(0.0);
					}
					if let Some(speed) = speed {
						settings.wind.speed = speed.max(0.0);
					}
				});
				*console = "wind updated".into();
			}
		}
	}
}

impl LodCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		let Self::Host { level } = self;
		let lod = match level {
			HostLod::High => GrassPatchLod::High,
			HostLod::Medium => GrassPatchLod::Medium,
			HostLod::Low => GrassPatchLod::Low,
			HostLod::UltraLow => GrassPatchLod::UltraLow,
		};
		commands.queue(move |world: &mut World| {
			let mut patches = world.query::<&mut GrassPatch>();
			for mut patch in patches.iter_mut(world) {
				patch.lod = lod;
			}
		});
		*console = format!("host lod {level:?}");
	}
}

impl ShadowCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		let mode = match self {
			Self::Proxy => CanopyShadowMode::Proxy,
			Self::Full => CanopyShadowMode::Full,
			Self::Off => CanopyShadowMode::Off,
		};
		commands.insert_resource(mode);
		*console = format!("shadows {}", mode.label());
	}
}

impl CompareCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Turf { visible } => {
				let show = matches!(visible, VisibilityChoice::On);
				commands.queue(move |world: &mut World| {
					let mut planes = world.query_filtered::<&mut Visibility, With<TurfPlane>>();
					for mut visibility in planes.iter_mut(world) {
						*visibility = if show { Visibility::Inherited } else { Visibility::Hidden };
					}
				});
				*console = format!("turf {}", if show { "on" } else { "off" });
			}
			Self::Grass { visible } => {
				if matches!(visible, VisibilityChoice::Off) {
					commands.queue(|world: &mut World| despawn_patches(world));
					*console = "grass off".into();
				} else {
					SpawnCommand::Default.react(commands, console);
				}
			}
		}
	}
}

fn despawn_patches(world: &mut World) {
	let mut patches = world.query_filtered::<Entity, With<GrassPatch>>();
	let ids: Vec<_> = patches.iter(world).collect();
	for entity in ids {
		world.entity_mut(entity).despawn();
	}
}

impl GameCommand for PlaygroundCommand {
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}

impl std::fmt::Debug for HostLod {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(match self {
			Self::High => "high",
			Self::Medium => "medium",
			Self::Low => "low",
			Self::UltraLow => "ultra-low",
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_spawn_and_camera() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("spawn meadow")?;
		assert!(matches!(command, PlaygroundCommand::Spawn(SpawnCommand::Meadow)));
		let command = <PlaygroundCommand as GameCommand>::parse_line("camera ground")?;
		assert!(matches!(command, PlaygroundCommand::Camera(CameraCommand::Ground)));
		let command = <PlaygroundCommand as GameCommand>::parse_line("wind set --strength 0.3")?;
		assert!(matches!(command, PlaygroundCommand::Wind(WindCommand::Set { .. })));
		Ok(())
	}
}
