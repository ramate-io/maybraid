use bevy::prelude::*;
use durham_terrain_models::TerrainCellLayout;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::player::PlaygroundMode;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Durham terrain — / for commands — Y or F1 drawer".into(),
		empty_console_text:
			"Console: `set-character braidman`, `mode character`, `stats mesh`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint:
			"help — set-character <species> — mode free|character — cells show|set — stats mesh"
				.into(),
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
