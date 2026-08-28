use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "World — Durham + forest + sky — / for commands — Y or F1 drawer".into(),
		empty_console_text: "Console: `stats mesh`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint: "help — stats mesh — WASD fly".into(),
	}
}

pub(crate) fn sync_command_status_text(
	mut status: ResMut<GameCommandStatusText>,
	mut seeded: Local<bool>,
) {
	if *seeded {
		return;
	}
	*seeded = true;
	status.0 = "world  forest hopscotch  present 1 km  generate 2 km  sky dome 2 km".into();
}
