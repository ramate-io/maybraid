//! Sphere catalog for world [`material_ref::MaterialRef`] recipes,
//! plus mythical-chain preview commands.

mod camera;
mod capture;
mod chain_capture;
mod chain_lighting;
mod chain_preview;
pub mod commands;
mod ground;
mod preview;
mod shape;
mod shoot;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use preview::PreviewConfig;

use bevy::prelude::*;
use chain_capture::{
	tick_capture as tick_chain_capture, tick_video as tick_chain_video, CaptureSet, VideoCapture,
};
use chain_lighting::{setup_lighting, sync_lighting};
use chain_preview::{
	apply_pending_hit, pose_skinned_tube, sync_preview as sync_chain_preview, sync_visual_style,
	tick_sweep, ChainPreview, LightingMode, PendingChainHit,
};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::GameCommandDrawerConfig;
use ground::setup_ground;
use maybraid_world::VisualStylePlugin;
use preview::{sync_preview, PreviewHost};
use shape::setup_meshes;
use shoot::{fly_shots, tick_shoot, ShootConfig};
use world_materials::WorldMaterialsPlugin;

pub struct WorldMaterialsPlaygroundPlugin;

impl Plugin for WorldMaterialsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		let capturing = capture::CaptureSession::from_env();
		let mut command_plugin =
			GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config());
		if capturing.is_some() {
			command_plugin = command_plugin
				.with_drawer_config(GameCommandDrawerConfig { open_at_start: false, ..default() });
		}
		app.init_resource::<PreviewConfig>()
			.init_resource::<PreviewHost>()
			.init_resource::<ShootConfig>()
			.init_resource::<ChainPreview>()
			.init_resource::<LightingMode>()
			.init_resource::<PendingChainHit>()
			.init_resource::<CaptureSet>()
			.init_resource::<VideoCapture>()
			.add_plugins(WorldMaterialsPlugin)
			.add_plugins(VisualStylePlugin)
			.add_plugins(command_plugin);
		if let Some(session) = capturing {
			app.insert_resource(session).add_systems(
				Startup,
				(
					capture::apply_column_preview,
					capture::hide_capture_hud.after(game_commands::ui::setup_debug_ui),
				),
			);
		}
		app.add_systems(
			Startup,
			(camera::setup_camera, setup_lighting, setup_ground, setup_meshes),
		)
		.add_systems(
			Update,
			(
				camera::release_modifiers_on_focus_change.before(camera::camera_controller),
				camera::camera_controller.run_if(not(resource_exists::<capture::CaptureSession>)),
				camera::apply_host_camera_frame
					.run_if(not(resource_exists::<capture::CaptureSession>)),
				sync_lighting,
				sync_visual_style,
				apply_pending_hit,
				tick_sweep,
				pose_skinned_tube,
				sync_preview.after(capture_command_line_input::<PlaygroundCommand>),
				sync_chain_preview.after(capture_command_line_input::<PlaygroundCommand>),
				tick_shoot.after(sync_preview),
				fly_shots,
				tick_chain_capture.after(sync_chain_preview),
				tick_chain_video.after(sync_chain_preview),
				chain_capture::hide_hud_during_evidence,
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
			),
		);
		if app.world().contains_resource::<capture::CaptureSession>() {
			app.add_systems(Update, capture::tick_capture.after(sync_preview));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use clap::Parser;
	use game_commands::command::GameCommand;
	use world_materials::chain::MythicalChainState;

	#[test]
	fn parses_show_hex() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show hex")?;
		let PlaygroundCommand::Show { recipe } = command else {
			return Err("expected show".into());
		};
		assert_eq!(recipe, "hex");
		Ok(())
	}

	#[test]
	fn parses_shoot() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("shoot")?;
		assert!(matches!(command, PlaygroundCommand::Shoot { action: None }));
		Ok(())
	}

	#[test]
	fn parses_shoot_stop() -> Result<(), String> {
		let command = PlaygroundCommand::try_parse_from(["materials", "shoot", "stop"])
			.map_err(|err| err.to_string())?;
		assert!(matches!(
			command,
			PlaygroundCommand::Shoot { action: Some(commands::ShootAction::Stop) }
		));
		Ok(())
	}

	#[test]
	fn parses_mesh_blast() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh blast")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Blast);
		Ok(())
	}

	#[test]
	fn parses_mesh_ring() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh ring")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Ring);
		Ok(())
	}

	#[test]
	fn parses_mesh_cylinder() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh cylinder")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Cylinder);
		Ok(())
	}

	#[test]
	fn parses_mesh_taper() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh taper")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Taper);
		Ok(())
	}

	#[test]
	fn show_doric_uses_the_column_recipe() -> Result<(), String> {
		let material = commands::material_for_show("doric");
		assert_eq!(material.name, material_ref::MaterialId::named("doric_column"));
		Ok(())
	}

	#[test]
	fn parses_chain_show_pair() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("chain show pair")?;
		let PlaygroundCommand::Chain { action: commands::ChainCommand::Show { state } } = command
		else {
			return Err("expected chain show".into());
		};
		assert_eq!(state, "pair");
		Ok(())
	}

	#[test]
	fn parses_chain_near_breaking_and_night() -> Result<(), String> {
		let show = <PlaygroundCommand as GameCommand>::parse_line("chain show near-breaking")?;
		assert!(matches!(
			show,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Show { state } }
				if state == "near-breaking"
		));
		let lighting = <PlaygroundCommand as GameCommand>::parse_line("chain lighting night")?;
		assert!(matches!(
			lighting,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Lighting { mode } }
				if mode == "night"
		));
		assert_eq!(
			MythicalChainState::from_label("near-breaking"),
			Some(MythicalChainState::NearBreaking)
		);
		Ok(())
	}

	#[test]
	fn parses_chain_sweep_stop() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("chain sweep stop")?;
		assert!(matches!(
			command,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Sweep { action: Some(action) } }
				if action == "stop"
		));
		Ok(())
	}

	#[test]
	fn parses_chain_capture_dir() -> Result<(), String> {
		let command =
			<PlaygroundCommand as GameCommand>::parse_line("chain capture /tmp/chain-previews")?;
		assert!(matches!(
			command,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Capture { dir: Some(dir) } }
				if dir.ends_with("chain-previews")
		));
		Ok(())
	}

	#[test]
	fn parses_chain_hit_style_and_video() -> Result<(), String> {
		let hit = <PlaygroundCommand as GameCommand>::parse_line("chain hit 6")?;
		assert!(matches!(
			hit,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Hit { link: 6 } }
		));
		let style = <PlaygroundCommand as GameCommand>::parse_line("chain style retro")?;
		assert!(matches!(
			style,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Style { mode } }
				if mode == "retro"
		));
		let video = <PlaygroundCommand as GameCommand>::parse_line("chain video /tmp/chain-video")?;
		assert!(matches!(
			video,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Video { dir } }
				if dir.ends_with("chain-video")
		));
		Ok(())
	}

	#[test]
	fn parses_show_chain_alias() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show chain")?;
		let PlaygroundCommand::Show { recipe } = command else {
			return Err("expected show".into());
		};
		assert_eq!(recipe, "chain");
		Ok(())
	}
}
