use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::{GroveKind, PlaygroundConfig};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Vegetation on terrain — / for commands — Y or F1 drawer".into(),
		empty_console_text: "Console: `grove rolling-oaks`, `stats mesh`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint:
			"help — grove <kind> — terrain-radius — grove-extent — tile-radius — stats mesh".into(),
	}
}

pub(crate) fn sync_command_status_text(
	config: Res<PlaygroundConfig>,
	mut status: ResMut<GameCommandStatusText>,
	mut last: Local<Option<(GroveKind, i32, u32, i32)>>,
) {
	// Keep `stats mesh` (and other one-shot status) until config changes.
	let extent_bits = config.grove_extent_xz.to_bits();
	let key = (config.grove, config.terrain_radius, extent_bits, config.tile_radius);
	if *last == Some(key) {
		return;
	}
	*last = Some(key);
	status.0 = format!(
		"grove={}  terrain-radius={}  grove-extent={:.0}  tile-radius={}",
		config.grove.label(),
		config.terrain_radius,
		config.grove_extent_xz,
		config.tile_radius
	);
}
