use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::render::RenderConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "SBS trees playground - / cmd - WASD - up/down history - PgUp/PgDn scroll".into(),
		empty_console_text:
			"Console: `render …`, `stats mesh`, `help` — wheel or PgUp/PgDn".into(),
		root_background: Color::srgba(0.1, 0.2, 0.24, 0.82),
		controls_hint: "help — render … — stats mesh — Enter — history — PgUp/PgDn".into(),
	}
}

pub(crate) fn sync_command_status_text(
	config: Res<RenderConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = format!("{} render  res_2={}", config.subject.label(), config.res_2);
}
