use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::preview::PreviewConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title:
			"SDF playground - Tab/1-6 - +/- res - / cmd - WASD - up/down history - PgUp/PgDn scroll"
				.into(),
		empty_console_text: "Console: (errors & `help` output) - wheel or PgUp/PgDn".into(),
		root_background: Color::hsla(201.0, 0.69, 0.62, 0.82),
		controls_hint: "help - Enter - up/down history - PgUp/PgDn - Shift+up/down scroll".into(),
	}
}

pub(crate) fn sync_command_status_text(
	config: Res<PreviewConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = format!(
		"{} ({})  res_2={}",
		config.primitive,
		config.primitive.variant_key(),
		config.res_2,
	);
}
