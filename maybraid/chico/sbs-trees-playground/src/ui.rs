use bevy::prelude::*;
use chico_forests::{ForestStream, ForestStreamSpec};
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "SBS forest playground - / cmd - WASD - up/down history - PgUp/PgDn scroll".into(),
		empty_console_text: "Console: `forest`, `stats mesh`, `help` — wheel or PgUp/PgDn".into(),
		root_background: Color::srgba(0.1, 0.2, 0.24, 0.82),
		controls_hint: "help — forest — stats mesh — Enter — history — PgUp/PgDn".into(),
	}
}

pub(crate) fn sync_command_status_text(
	stream: Res<ForestStream>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = match stream.0.unwrap_or_default() {
		ForestStreamSpec { layering, stream_radius, .. } => {
			let layering = layering.map(|k| k.as_kebab()).unwrap_or("hopscotch");
			format!("forest {layering}  stream-radius={stream_radius}")
		}
	};
}
