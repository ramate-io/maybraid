//! In-game clap commands for the tree canopy playground.

use bevy::prelude::*;
use clap::{Parser, Subcommand, ValueEnum};
use game_commands::command::{CommandScript, GameCommand};
use vegetation_components::CanopyShadowMode;
use vegetation_tree_field::{LeafCloseStyle, TreeFieldSettings, TreePatch, TreePatchLod};

use crate::camera::{yaw_pitch, CameraController};
use crate::ground::TurfPlane;
use crate::kit;
use crate::scene::{self, PatchKind};

pub const PLAYGROUND_CLI_NAME: &str = "tree-field";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "tree-field",
	version,
	about = "GPU tree-canopy playground commands (in-game after `/` or process argv)",
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
	Leaves {
		#[arg(value_enum)]
		style: LeavesStyle,
	},
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum SpawnCommand {
	Default,
	Close,
	Grove,
	Slope,
	Leafless,
	Sparse,
	Lod,
	Willow,
	Forest,
	Walk,
	Clear,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum CameraCommand {
	Orbit,
	Overhead,
	Underside,
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
	Gpu { visible: VisibilityChoice },
	Kit { visible: VisibilityChoice },
}

#[derive(Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum VisibilityChoice {
	On,
	Off,
}

#[derive(Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum LeavesStyle {
	Smooth,
	Dither,
	Crisp,
}

impl LeavesStyle {
	fn close_style(self) -> LeafCloseStyle {
		match self {
			Self::Smooth => LeafCloseStyle::Smooth,
			Self::Dither => LeafCloseStyle::Dither,
			Self::Crisp => LeafCloseStyle::Crisp,
		}
	}
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
			Self::Leaves { style } => {
				let close = style.close_style();
				commands.queue(move |world: &mut World| {
					world.resource_mut::<TreeFieldSettings>().leaf_close_style = Some(close);
					let mut patches = world.query::<&mut TreePatch>();
					for mut patch in patches.iter_mut(world) {
						patch.recipe.leaf_close_style = close;
					}
				});
				*console = format!("leaves {}", close.label());
			}
		}
	}
}

impl SpawnCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Default => {
				commands.queue(|world: &mut World| {
					kit::clear_kit(world);
					scene::clear_patches(world);
					scene::spawn_default_scene_in_world(world);
				});
				*console = "spawned default grove + close + slope + leafless".into();
			}
			Self::Clear => {
				commands.queue(|world: &mut World| {
					kit::clear_kit(world);
					scene::clear_patches(world);
				});
				*console = "cleared tree patches".into();
			}
			other => {
				let kind = match other {
					Self::Close => PatchKind::Close,
					Self::Grove => PatchKind::Grove,
					Self::Slope => PatchKind::Slope,
					Self::Leafless => PatchKind::Leafless,
					Self::Sparse => PatchKind::Sparse,
					Self::Lod => PatchKind::Lod,
					Self::Willow => PatchKind::Willow,
					Self::Forest => PatchKind::WorldForest,
					Self::Walk => PatchKind::WorldWalk,
					Self::Default | Self::Clear => unreachable!(),
				};
				commands.queue(move |world: &mut World| {
					kit::clear_kit(world);
					scene::clear_patches(world);
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
			Self::Orbit => (Vec3::new(16.5, 8.2, 18.0), Vec3::new(0.0, 7.0, 0.0)),
			Self::Overhead => (Vec3::new(1.4, 28.0, 1.1), Vec3::new(0.0, 0.0, 0.0)),
			Self::Underside => (Vec3::new(3.2, 1.55, 4.6), Vec3::new(0.2, 10.4, 0.0)),
			Self::Ground => (Vec3::new(2.0, 9.5, 48.0), Vec3::new(22.0, 8.0, 22.0)),
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
			Self::Underside => "camera underside".into(),
			Self::Ground => "camera ground".into(),
		};
	}
}

impl WindCommand {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Still => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<TreeFieldSettings>().wind.strength_m = 0.0;
				});
				*console = "wind still".into();
			}
			Self::Gentle => {
				commands.queue(|world: &mut World| {
					world.resource_mut::<TreeFieldSettings>().wind =
						vegetation_shaders::wesl::WindInputs::gentle_botw(0.0);
				});
				*console = "wind gentle BotW".into();
			}
			Self::Set { strength, speed } => {
				commands.queue(move |world: &mut World| {
					let mut settings = world.resource_mut::<TreeFieldSettings>();
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
			HostLod::High => TreePatchLod::High,
			HostLod::Medium => TreePatchLod::Medium,
			HostLod::Low => TreePatchLod::Low,
			HostLod::UltraLow => TreePatchLod::UltraLow,
		};
		commands.queue(move |world: &mut World| {
			let mut patches = world.query::<&mut TreePatch>();
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
			Self::Gpu { visible } => {
				if matches!(visible, VisibilityChoice::Off) {
					commands.queue(|world: &mut World| scene::clear_patches(world));
					*console = "gpu off".into();
				} else {
					SpawnCommand::Default.react(commands, console);
				}
			}
			Self::Kit { visible } => {
				if matches!(visible, VisibilityChoice::Off) {
					commands.queue(|world: &mut World| kit::clear_kit(world));
					*console = "kit off".into();
				} else {
					commands.queue(|world: &mut World| kit::spawn_kit_from_patches(world));
					*console = "kit on".into();
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
		let command = <PlaygroundCommand as GameCommand>::parse_line("spawn grove")?;
		assert!(matches!(command, PlaygroundCommand::Spawn(SpawnCommand::Grove)));
		let command = <PlaygroundCommand as GameCommand>::parse_line("camera underside")?;
		assert!(matches!(command, PlaygroundCommand::Camera(CameraCommand::Underside)));
		let command = <PlaygroundCommand as GameCommand>::parse_line("wind set --strength 0.4")?;
		assert!(matches!(command, PlaygroundCommand::Wind(WindCommand::Set { .. })));
		let command = <PlaygroundCommand as GameCommand>::parse_line("leaves dither")?;
		assert!(matches!(command, PlaygroundCommand::Leaves { style: LeavesStyle::Dither }));
		let command = <PlaygroundCommand as GameCommand>::parse_line("leaves smooth")?;
		assert!(matches!(command, PlaygroundCommand::Leaves { style: LeavesStyle::Smooth }));
		let command = <PlaygroundCommand as GameCommand>::parse_line("leaves crisp")?;
		assert!(matches!(command, PlaygroundCommand::Leaves { style: LeavesStyle::Crisp }));
		Ok(())
	}
}
