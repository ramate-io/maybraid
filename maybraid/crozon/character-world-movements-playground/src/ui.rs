use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::player::PlaygroundMode;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Crozon movements — / for commands — Y or F1 drawer".into(),
		empty_console_text: "Console: `set-character braidman`, `stampede`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint: "help — set-character <species> — stampede — mode free|character".into(),
	}
}

pub(crate) fn sync_command_status_text(
	mode: Res<PlaygroundMode>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let mode_label = match *mode {
		PlaygroundMode::Free => "free",
		PlaygroundMode::Character => "character",
	};
	status.0 = format!("mode={mode_label}  WASD move  Space jump");
}
