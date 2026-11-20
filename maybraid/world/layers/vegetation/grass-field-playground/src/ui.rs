use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use vegetation_grass_field::{GrassField, GrassFieldSettings};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "GPU grass fields - / cmd - L look - WASD - Space/Shift up/down".into(),
		empty_console_text: "Console: `spawn parting`, `parting world`, `camera ground`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.10, 0.82),
		controls_hint: "help — spawn — camera — wind — lod — shadows — compare — parting".into(),
	}
}

pub(crate) fn sync_command_status_text(
	field: Res<GrassField>,
	settings: Res<GrassFieldSettings>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = format!(
		"blades {} | patches {} | wind {:.2} m @ {:.2}\n\
		 parting {} str {:.2} r {:.2} falloff {:.2}",
		field.blades.len(),
		field.patches.len(),
		settings.wind.strength_m,
		settings.wind.speed,
		settings.parting.mode.label(),
		settings.parting.strength,
		settings.parting.radius,
		settings.parting.depth_falloff,
	);
}
