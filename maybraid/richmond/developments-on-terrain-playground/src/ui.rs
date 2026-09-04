use bevy::prelude::*;
use durham_terrain_models::{TerrainCellLayout, TerrainConfig};
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use richmond_development_models::DevelopmentConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Developments on terrain — / for commands — Y or F1 drawer".into(),
		empty_console_text: "Console: `seed 7`, `likelihood 0.9`, `terrain-radius 2`, `rebuild`, `stats mesh`, `help`"
			.into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint: "help — seed <n> — likelihood <0-1> — terrain-radius <n> — rebuild — stats mesh"
			.into(),
	}
}

pub(crate) fn sync_command_status_text(
	layout: Res<TerrainCellLayout>,
	config: Res<TerrainConfig>,
	dev: Res<DevelopmentConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = format!(
		"seed={}  likelihood={:.2}  cells size={:.1} origin=({}, {}) extents={}×{}",
		config.seed,
		dev.likelihood,
		layout.cell_size,
		layout.origin.x,
		layout.origin.y,
		layout.extents.x,
		layout.extents.y
	);
}
