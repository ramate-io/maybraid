use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::playground_player::PlaygroundMode;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Routing on Durham — / for commands — Y or F1 drawer".into(),
		empty_console_text: "Console: `go <x> <z>`, `mode character`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint: "help — go <x> <z> — mode free|character".into(),
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
	status.0 = format!("mode={mode_label}  orange/yellow/cyan = coarse→fine  white = goal");
}

pub(crate) fn write_status(
	status: &mut Option<ResMut<GameCommandStatusText>>,
	text: impl Into<String>,
) {
	if let Some(status) = status {
		status.0 = text.into();
	}
}
