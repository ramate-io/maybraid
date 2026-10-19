//! Isolated host for the mythical-chain material.

mod camera;
mod capture;
pub mod commands;
mod ground;
mod lighting;
mod preview;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use preview::{LightingMode, PreviewConfig, PreviewStyle};

use bevy::prelude::*;
use capture::{tick_capture, tick_video, CaptureSet, VideoCapture};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use ground::setup_ground;
use lighting::{setup_lighting, sync_lighting};
use preview::{apply_pending_hit, sync_preview, sync_window_style, tick_sweep, PendingChainHit};
use world_materials::WorldMaterialsPlugin;

pub struct ChainPlaygroundPlugin;

impl Plugin for ChainPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PreviewConfig>()
			.init_resource::<LightingMode>()
			.init_resource::<PendingChainHit>()
			.init_resource::<CaptureSet>()
			.init_resource::<VideoCapture>()
			.add_plugins(WorldMaterialsPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()));
		app.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::release_modifiers_on_focus_change.before(camera::camera_controller),
					camera::camera_controller,
					sync_lighting,
					sync_window_style,
					apply_pending_hit,
					tick_sweep,
					sync_preview.after(capture_command_line_input::<PlaygroundCommand>),
					tick_capture.after(sync_preview),
					tick_video.after(sync_preview),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chain_shaders::MythicalChainState;
	use game_commands::command::GameCommand;

	#[test]
	fn parses_show_pair() -> anyhow::Result<()> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show pair")
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		let PlaygroundCommand::Show { state } = command else {
			return Err(anyhow::anyhow!("expected show"));
		};
		assert_eq!(state, "pair");
		Ok(())
	}

	#[test]
	fn parses_near_breaking_and_night() -> anyhow::Result<()> {
		let show = <PlaygroundCommand as GameCommand>::parse_line("show near-breaking")
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		assert!(matches!(show, PlaygroundCommand::Show { state } if state == "near-breaking"));
		let lighting = <PlaygroundCommand as GameCommand>::parse_line("lighting night")
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		assert!(matches!(lighting, PlaygroundCommand::Lighting { mode } if mode == "night"));
		assert_eq!(
			MythicalChainState::from_label("near-breaking"),
			Some(MythicalChainState::NearBreaking)
		);
		Ok(())
	}

	#[test]
	fn parses_sweep_stop() -> anyhow::Result<()> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("sweep stop")
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		assert!(
			matches!(command, PlaygroundCommand::Sweep { action: Some(action) } if action == "stop")
		);
		Ok(())
	}

	#[test]
	fn parses_capture_dir() -> anyhow::Result<()> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("capture /tmp/chain-previews")
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		assert!(
			matches!(command, PlaygroundCommand::Capture { dir } if dir.ends_with("chain-previews"))
		);
		Ok(())
	}

	#[test]
	fn parses_hit_style_and_video() -> anyhow::Result<()> {
		let hit = <PlaygroundCommand as GameCommand>::parse_line("hit 6")
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		assert!(matches!(hit, PlaygroundCommand::Hit { link: 6 }));
		let style = <PlaygroundCommand as GameCommand>::parse_line("style retro")
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		assert!(matches!(style, PlaygroundCommand::Style { mode } if mode == "retro"));
		let video = <PlaygroundCommand as GameCommand>::parse_line("video /tmp/chain-video")
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		assert!(matches!(video, PlaygroundCommand::Video { dir } if dir.ends_with("chain-video")));
		Ok(())
	}
}
