use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "VFX — / cmd — WASD/Space/Shift — mouse look".into(),
		empty_console_text: "Console: `help`".into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "help — Enter — history".into(),
	}
}

pub(crate) fn sync_command_status_text(mut status: ResMut<GameCommandStatusText>) {
	if status.0.is_empty() {
		status.0 = "vfx: (empty — concepts land here)".into();
	}
}
