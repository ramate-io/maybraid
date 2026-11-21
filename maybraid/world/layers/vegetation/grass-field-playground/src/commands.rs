//! In-game clap commands for the grass field playground.

use bevy::prelude::*;
use clap::{Parser, Subcommand, ValueEnum};
use game_commands::command::{CommandScript, GameCommand};
use vegetation_components::CanopyShadowMode;
use vegetation_grass_field::{
	GrassFieldSettings, GrassPartingDebug, GrassPartingMode, GrassPatch, GrassPatchLod,
};

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
	#[command(subcommand)]
	Parting(PartingCommand),
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum SpawnCommand {
	Default,
	Clump,
	Meadow,
	Slope,
	Crest,
	Parting,
	Clear,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum CameraCommand {
	Orbit,
	Overhead,
	Ground,
	Topdown,
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
pub enum PartingCommand {
	Off,
	Depth,
	Raster,
	World,
	Set {
		#[arg(long)]
		strength: Option<f32>,
		#[arg(long)]
		radius: Option<f32>,
		#[arg(long)]
		falloff: Option<f32>,
		#[arg(long)]
		stamp_scale: Option<f32>,
	},
	#[command(subcommand)]
	Debug(PartingDebugCommand),
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PartingDebugCommand {
	/// Tint a ground quad with raster occupancy (magenta disc, 2.5× radius).
	Mask,
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
			Self::Parting(command) => command.react(commands, console),
		}
	}
}

impl SpawnCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Default => {
				commands.queue(|world: &mut World| {
					scene::clear_parting_props(world);
					scene::clear_patches(world);
					scene::spawn_default_scene_in_world(world);
				});
				*console = "spawned default meadow + clump + slope + crest".into();
			}
			Self::Clear => {
				commands.queue(|world: &mut World| {
					scene::clear_parting_props(world);
					scene::clear_patches(world);
				});
				*console = "cleared grass patches".into();
			}
			Self::Parting => {
				commands.queue(|world: &mut World| {
					scene::spawn_parting_scene_in_world(world, 1.35, 0.95);
				});
				*console = "spawned parting corridor (capsule + rock)".into();
			}
			other => {
				let kind = match other {
					Self::Clump => PatchKind::Clump,
					Self::Meadow => PatchKind::Meadow,
					Self::Slope => PatchKind::Slope,
					Self::Crest => PatchKind::Crest,
					Self::Default | Self::Clear | Self::Parting => unreachable!(),
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
			Self::Topdown => scene::PARTING_TOPDOWN,
		};
		commands.queue(move |world: &mut World| {
			let mut cameras =
				world.query_filtered::<(&mut Transform, &mut CameraController), With<Camera3d>>();
			if let Ok((mut transform, mut controller)) = cameras.single_mut(world) {
				let dir = (look - from).normalize_or(Vec3::NEG_Y);
				let up = if dir.dot(Vec3::Y).abs() > 0.95 { Vec3::NEG_Z } else { Vec3::Y };
				*transform = Transform::from_translation(from).looking_at(look, up);
				let (yaw, pitch) = yaw_pitch(transform.rotation);
				controller.yaw = yaw;
				controller.pitch = pitch;
			}
		});
		*console = match self {
			Self::Orbit => "camera orbit".into(),
			Self::Overhead => "camera overhead".into(),
			Self::Ground => "camera ground".into(),
			Self::Topdown => "camera topdown".into(),
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

impl PartingCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Off | Self::Depth | Self::Raster | Self::World => {
				let mode = match self {
					Self::Off => GrassPartingMode::Off,
					Self::Depth => GrassPartingMode::Depth,
					Self::Raster => GrassPartingMode::Raster,
					Self::World => GrassPartingMode::World,
					Self::Set { .. } | Self::Debug(_) => unreachable!(),
				};
				commands.queue(move |world: &mut World| {
					world.resource_mut::<GrassFieldSettings>().parting.mode = mode;
				});
				*console = format!("parting {}", mode.label());
			}
			Self::Set { strength, radius, falloff, stamp_scale } => {
				commands.queue(move |world: &mut World| {
					let mut settings = world.resource_mut::<GrassFieldSettings>();
					if let Some(strength) = strength {
						settings.parting.strength = strength.max(0.0);
					}
					if let Some(radius) = radius {
						settings.parting.radius = radius.max(0.0);
					}
					if let Some(falloff) = falloff {
						settings.parting.depth_falloff = falloff.max(0.05);
					}
					if let Some(stamp_scale) = stamp_scale {
						settings.parting.stamp_scale = stamp_scale.max(0.05);
					}
				});
				*console = "parting settings updated".into();
			}
			Self::Debug(PartingDebugCommand::Mask) => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<GrassPartingDebug>().mask_overlay = true;
				});
				*console = "parting debug mask overlay on".into();
			}
			Self::Debug(PartingDebugCommand::Off) => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<GrassPartingDebug>().mask_overlay = false;
				});
				*console = "parting debug overlay off".into();
			}
		}
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
					commands.queue(|world: &mut World| scene::clear_patches(world));
					*console = "grass off".into();
				} else {
					SpawnCommand::Default.react(commands, console);
				}
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
		let command = <PlaygroundCommand as GameCommand>::parse_line("camera topdown")?;
		assert!(matches!(command, PlaygroundCommand::Camera(CameraCommand::Topdown)));
		let command = <PlaygroundCommand as GameCommand>::parse_line("wind set --strength 0.3")?;
		assert!(matches!(command, PlaygroundCommand::Wind(WindCommand::Set { .. })));
		let command = <PlaygroundCommand as GameCommand>::parse_line("parting depth")?;
		assert!(matches!(command, PlaygroundCommand::Parting(PartingCommand::Depth)));
		let command = <PlaygroundCommand as GameCommand>::parse_line("spawn parting")?;
		assert!(matches!(command, PlaygroundCommand::Spawn(SpawnCommand::Parting)));
		let command =
			<PlaygroundCommand as GameCommand>::parse_line("parting set --stamp-scale 2.5")?;
		assert!(matches!(
			command,
			PlaygroundCommand::Parting(PartingCommand::Set { stamp_scale: Some(scale), .. })
				if (scale - 2.5).abs() < 1e-5
		));
		let command = <PlaygroundCommand as GameCommand>::parse_line("parting debug mask")?;
		assert!(matches!(
			command,
			PlaygroundCommand::Parting(PartingCommand::Debug(PartingDebugCommand::Mask))
		));
		Ok(())
	}
}
