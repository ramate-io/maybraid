use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use menu_screens::{HomeScreen, InGameScreen, LoadingScreen};

use crate::character::CharacterScreen;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Maybraid menu playground — / for commands — Y or F1 drawer".into(),
		empty_console_text: "Console: `show home`, `show in-game`, `show character`, `help`".into(),
		root_background: Color::srgba(0.08, 0.10, 0.14, 0.82),
		controls_hint: "arrows select — Enter choose — Y or F1 drawer — / cmd".into(),
	}
}

pub(crate) fn sync_command_status_text(
	home: Query<(), With<HomeScreen>>,
	in_game: Query<(), With<InGameScreen>>,
	loading: Query<(), With<LoadingScreen>>,
	character: Query<(), With<CharacterScreen>>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = if !home.is_empty() {
		"screen=home".into()
	} else if !in_game.is_empty() {
		"screen=in-game".into()
	} else if !loading.is_empty() {
		"screen=loading".into()
	} else if !character.is_empty() {
		"screen=character".into()
	} else {
		"no screen".into()
	};
}
