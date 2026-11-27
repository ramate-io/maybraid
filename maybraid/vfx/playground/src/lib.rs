//! Isolated host for real-time VFX concepts.

mod camera;
pub mod commands;
mod ground;
mod inspector;
mod preview;
pub use preview::{PreviewRecipes, VfxPreviewAppExt};
mod show;
mod ui;
mod volume_demo;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use game_commands::command::GameCommandPlugin;
use ground::setup_ground;
use maybraid_vfx::effects::cartoon_dust::CartoonDust;
use maybraid_vfx::VfxPlugin;
use show::{
	apply_pending_shows, pause_at_frozen_age, repeat_shown_effect, LastVfxShow, RepeatingVfxShow,
};

use crate::commands::{CameraOrbit, LoopEnabled, PendingVfxShows, PreviewFreeze, SpreadOnce};

pub struct VfxPlaygroundPlugin;

impl Plugin for VfxPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.register_vfx_preview("cartoon-dust", || CartoonDust)
			.register_vfx_preview("volume-demo", || volume_demo::VolumeDemo)
			.init_resource::<PreviewRecipes>()
			.init_resource::<preview::PreviewSelection>()
			.init_resource::<preview::PreviewStep>()
			.init_resource::<inspector::PreviewInspector>()
			.init_resource::<PendingVfxShows>()
			.init_resource::<LastVfxShow>()
			.init_resource::<RepeatingVfxShow>()
			.init_resource::<PreviewFreeze>()
			.init_resource::<LoopEnabled>()
			.init_resource::<CameraOrbit>()
			.init_resource::<SpreadOnce>()
			.add_plugins(VfxPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()));
		app.add_systems(First, preview::step_preview.after(bevy::time::TimeSystems))
			.add_observer(ui::scroll_layer_inspector)
			.add_systems(Last, inspector::inspect_preview)
			.add_systems(
				Startup,
				(camera::setup_camera, setup_lighting, setup_ground, ui::setup_layer_inspector),
			)
			.add_systems(
				Update,
				(
					camera::release_modifiers_on_focus_change.before(camera::camera_controller),
					camera::camera_controller,
					camera::orbit_camera.after(camera::camera_controller),
					apply_pending_shows,
					repeat_shown_effect.after(apply_pending_shows),
					pause_at_frozen_age.after(repeat_shown_effect),
					ui::sync_layer_inspector,
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight {
		brightness: 120.0,
		color: Color::srgb(0.55, 0.62, 0.78),
		..default()
	});
	commands.spawn((
		DirectionalLight {
			illuminance: 3200.0,
			color: Color::srgb(1.0, 0.92, 0.82),
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
}

#[cfg(test)]
mod tests {
	use super::*;
	use game_commands::command::GameCommand;
	use maybraid_vfx::{DUST, FIERY_EXPLOSION, FIREBALL, FLASH, SMOKE, SOUNDWAVE, SPARKS};

	#[test]
	fn parses_playback_controls() -> Result<(), String> {
		for line in [
			"pause", "play", "reset", "cancel", "spread", "reload", "solo 1", "mute 2", "unsolo",
			"unmute", "step", "step 0.1",
		] {
			<PlaygroundCommand as GameCommand>::parse_line(line)?;
		}
		Ok(())
	}

	#[test]
	fn rejects_invalid_steps() {
		for line in ["step NaN", "step inf", "step 0", "step -1", "step 1.1"] {
			assert!(<PlaygroundCommand as GameCommand>::parse_line(line).is_err(), "{line}");
		}
	}

	#[test]
	fn parses_help() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("help")?;
		assert!(matches!(command, PlaygroundCommand::Help));
		Ok(())
	}

	#[test]
	fn parses_show_default() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show")?;
		let PlaygroundCommand::Show { effect, scale, intensity, distance, seed, playback } =
			command
		else {
			return Err("expected show".into());
		};
		assert_eq!(effect, "fiery-explosion");
		assert!((scale - 1.0).abs() < 1e-4);
		assert!((intensity - 1.0).abs() < 1e-4);
		assert!(distance.abs() < 1e-4);
		assert!(seed.is_none());
		assert!((playback - 1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn parses_fiery_explosion_command() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("fiery-explosion")?;
		assert!(matches!(
			command,
			PlaygroundCommand::FieryExplosion { scale, intensity, distance, seed, playback }
				if (scale - 1.0).abs() < 1e-4
					&& (intensity - 1.0).abs() < 1e-4
					&& distance.abs() < 1e-4
					&& seed.is_none()
					&& (playback - 1.0).abs() < 1e-4
		));
		let command = <PlaygroundCommand as GameCommand>::parse_line(
			"fiery-explosion --scale 1.5 --intensity 1.2 --distance 6 --seed 3 --playback 0.5",
		)?;
		let PlaygroundCommand::FieryExplosion { scale, intensity, distance, seed, playback } =
			command
		else {
			return Err("expected fiery-explosion".into());
		};
		assert!((scale - 1.5).abs() < 1e-4);
		assert!((intensity - 1.2).abs() < 1e-4);
		assert!((distance - 6.0).abs() < 1e-4);
		assert_eq!(seed, Some(3));
		assert!((playback - 0.5).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn parses_show_layers_and_overrides() -> Result<(), String> {
		for (line, name) in [
			("show flash", FLASH),
			("show fireball", FIREBALL),
			("show smoke", SMOKE),
			("show dust", DUST),
			("show sparks", SPARKS),
			("show fiery-explosion", FIERY_EXPLOSION),
			("show soundwave", SOUNDWAVE),
			("show sonic-wave", SOUNDWAVE),
		] {
			let command = <PlaygroundCommand as GameCommand>::parse_line(line)?;
			let PlaygroundCommand::Show { effect, .. } = command else {
				return Err(format!("expected show for {line}"));
			};
			assert_eq!(maybraid_vfx::canonicalize_effect_name(&effect), Some(name));
		}
		let command = <PlaygroundCommand as GameCommand>::parse_line(
			"show sparks --scale 2 --intensity 1.5",
		)?;
		let PlaygroundCommand::Show { effect, scale, intensity, .. } = command else {
			return Err("expected show".into());
		};
		assert_eq!(effect, "sparks");
		assert!((scale - 2.0).abs() < 1e-4);
		assert!((intensity - 1.5).abs() < 1e-4);
		Ok(())
	}
}
