use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "World — character on Durham + forest + sky — / for commands — Y or F1 drawer"
			.into(),
		empty_console_text: "Console: `mode free`, `set-character`, `stats mesh`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint:
			"L-stick move — R-stick look — L3 sprint — R3 POV — A jump — LT focus — RT use — RT+X power — / commands"
				.into(),
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
	status.0 =
		"world  character  forest hopscotch  present 2 km  generate 3 km  sky 350–1200 m".into();
}
