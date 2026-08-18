use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::PlaygroundConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Vegetation on terrain — / for commands — Y or F1 drawer".into(),
		empty_console_text: "Console: `grove rolling-oaks`, `terrain-radius 2`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint: "help — grove <kind> — terrain-radius — grove-extent — tile-radius".into(),
	}
}

pub(crate) fn sync_command_status_text(
	config: Res<PlaygroundConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = format!(
		"grove={}  terrain-radius={}  grove-extent={:.0}  tile-radius={}",
		config.grove.label(),
		config.terrain_radius,
		config.grove_extent_xz,
		config.tile_radius
	);
}
