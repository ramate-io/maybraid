use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::preview::PreviewConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Mythical chain — / cmd — WASD/Space/Shift — mouse look".into(),
		empty_console_text: "Console: `show pair`, `lighting night`, `deform on`, `sweep`, `help`"
			.into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "show pair — lighting night — deform — sweep — Enter — history".into(),
	}
}

pub(crate) fn sync_command_status_text(
	preview: Res<PreviewConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = preview.status_label();
}
