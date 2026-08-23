use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use menu_screens::HomeScreen;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Maybraid menu playground — / for commands — Y or F1 drawer".into(),
		empty_console_text: "Console: `show home`, `help`".into(),
		root_background: Color::srgba(0.08, 0.10, 0.14, 0.82),
		controls_hint: "arrows select — Enter choose — Y or F1 drawer — / cmd".into(),
	}
}

pub(crate) fn sync_command_status_text(
	home: Query<(), With<HomeScreen>>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = if home.is_empty() { "no screen".into() } else { "screen=home".into() };
}
