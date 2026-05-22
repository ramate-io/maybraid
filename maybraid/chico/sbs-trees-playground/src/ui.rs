use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::preview::PreviewConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "SBS trees playground - / cmd - WASD - up/down history - PgUp/PgDn scroll".into(),
		empty_console_text: "Console: (errors & `help` output) - wheel or PgUp/PgDn".into(),
		root_background: Color::srgba(0.1, 0.2, 0.24, 0.82),
		controls_hint: "help - Enter - up/down history - PgUp/PgDn - Shift+up/down scroll".into(),
	}
}

pub(crate) fn sync_command_status_text(
	config: Res<PreviewConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = format!("{} preview  res_2={}", config.tree.label(), config.res_2);
}
