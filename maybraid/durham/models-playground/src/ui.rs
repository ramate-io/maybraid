use bevy::prelude::*;
use durham_terrain_models::TerrainCellLayout;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Durham terrain models — / cmd — WASD look — Space/Shift up/down".into(),
		empty_console_text: "Console: try `help` or `cells set --size 32 --extent-x 2 --extent-z 2`"
			.into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint: "help — cells show|set — Enter — up/down history — PgUp/PgDn".into(),
	}
}

pub(crate) fn sync_command_status_text(
	layout: Res<TerrainCellLayout>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = format!(
		"cells size={:.1} origin=({}, {}) extents={}×{}",
		layout.cell_size, layout.origin.x, layout.origin.y, layout.extents.x, layout.extents.y
	);
}
