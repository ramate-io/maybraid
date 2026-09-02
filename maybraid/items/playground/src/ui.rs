use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::preview::PreviewConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Items playground - / cmd - L look - WASD - Space up - Shift down".into(),
		empty_console_text:
			"Console: `show bullpup`, `kit --barrel laznard`, `scale barrel --length 1.5`, `help`"
				.into(),
		root_background: Color::srgba(0.12, 0.14, 0.18, 0.82),
		controls_hint:
			"help — show — kit --body/--barrel/--grip — scale <bone> --length/--thickness".into(),
	}
}

pub(crate) fn sync_command_status_text(
	config: Res<PreviewConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = format!("firearm {} | scale {}", config.kit.label(), config.pose.label());
}
