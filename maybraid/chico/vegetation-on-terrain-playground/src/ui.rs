use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::{GroveKind, PlaygroundConfig};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Vegetation on terrain — / for commands — Y or F1 drawer".into(),
		empty_console_text:
			"Console: `grove rolling-oaks`, `forest`, `mode character`, `stats mesh`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint:
			"help — grove <kind> — forest [layering] — mode free|character — set-character — stats mesh".into(),
	}
}

pub(crate) fn sync_command_status_text(
	config: Res<PlaygroundConfig>,
	mut status: ResMut<GameCommandStatusText>,
	mut last: Local<Option<(GroveKind, i32, u32, i32, Option<String>)>>,
) {
	// Keep `stats mesh` (and other one-shot status) until config changes.
	let extent_bits = config.grove_extent_xz.to_bits();
	let forest_key = config.forest.map(|spec| spec.key());
	let key =
		(config.grove, config.terrain_radius, extent_bits, config.tile_radius, forest_key.clone());
	if *last == Some(key.clone()) {
		return;
	}
	*last = Some(key);
	status.0 = match &config.forest {
		Some(spec) => {
			let layering = spec.layering.map(|k| k.as_kebab()).unwrap_or("hopscotch");
			format!(
				"forest {layering}  stream-radius={}  terrain-radius={}",
				spec.stream_radius, config.terrain_radius
			)
		}
		None => format!(
			"grove={}  terrain-radius={}  grove-extent={:.0}  tile-radius={}",
			config.grove.label(),
			config.terrain_radius,
			config.grove_extent_xz,
			config.tile_radius
		),
	};
}

pub(crate) fn write_status(
	status: &mut Option<ResMut<GameCommandStatusText>>,
	text: impl Into<String>,
) {
	if let Some(status) = status {
		status.0 = text.into();
	}
}
