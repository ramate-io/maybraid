use bevy::prelude::*;
use durham_terrain_models::TerrainCellLayout;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::player::PlaygroundMode;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Durham terrain — / cmd — mode free|character".into(),
		empty_console_text:
			"Console: `mode character`, `cells set --extent-x 3 --extent-z 3`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint: "help — mode free|character — cells show|set — Enter — history".into(),
	}
}

pub(crate) fn sync_command_status_text(
	layout: Res<TerrainCellLayout>,
	mode: Res<PlaygroundMode>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let mode_label = match *mode {
		PlaygroundMode::Free => "free",
		PlaygroundMode::Character => "character",
	};
	status.0 = format!(
		"mode={mode_label}  cells size={:.1} origin=({}, {}) extents={}×{}",
		layout.cell_size, layout.origin.x, layout.origin.y, layout.extents.x, layout.extents.y
	);
}
