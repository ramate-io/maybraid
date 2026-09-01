use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use firearms::WeaponsArmed;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Firing range - / cmd - L look - WASD - Space up - Shift down".into(),
		empty_console_text: "Console: `pause`, `resume`, `help`".into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "help — pause — resume — Enter — history".into(),
	}
}

pub(crate) fn sync_command_status_text(
	armed: Res<WeaponsArmed>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let fire = if armed.0 { "firing" } else { "paused" };
	status.0 = format!("range bolt / bullet / laser | {fire}");
}
