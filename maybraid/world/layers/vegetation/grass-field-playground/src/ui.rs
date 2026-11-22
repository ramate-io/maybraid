use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use vegetation_grass_field::{GrassFieldSettings, GrassGroveCatalog};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "GPU grass fields - / cmd - L look - WASD - Space/Shift up/down".into(),
		empty_console_text: "Console: `spawn meadow`, `camera ground`, `wind set`, `help`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.10, 0.82),
		controls_hint: "help — spawn — camera — wind — lod — shadows — compare".into(),
	}
}

pub(crate) fn sync_command_status_text(
	catalog: Res<GrassGroveCatalog>,
	settings: Res<GrassFieldSettings>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = format!(
		"blades {} | patches {} | wind {:.2} m @ {:.2}\n\
		 GPU select+indirect | nested LOD | BotW meadow palette",
		catalog.blades.iter().filter(|blade| blade.height_m > 0.0).count(),
		catalog.live_grove_count(),
		settings.wind.strength_m,
		settings.wind.speed,
	);
}
