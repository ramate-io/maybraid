use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use vegetation_tree_field::{TreeFieldSettings, TreeGroveCatalog};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "GPU tree canopies - / cmd - L look - WASD - Space/Shift up/down".into(),
		empty_console_text: "Console: `spawn grove`, `leaves dither`, `camera orbit`, `help`"
			.into(),
		root_background: Color::srgba(0.10, 0.16, 0.08, 0.82),
		controls_hint: "help — spawn — leaves — camera — wind — lod — shadows — compare".into(),
	}
}

pub(crate) fn sync_command_status_text(
	catalog: Res<TreeGroveCatalog>,
	settings: Res<TreeFieldSettings>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let style = settings.leaf_close_style.unwrap_or_default();
	status.0 = format!(
		"plants {} | wood {} | leaves {} | wind {:.2} m | leaves {}\n\
		 GPU wood+foliage | nested LOD | BotW canopy palette",
		catalog.resident_plants(),
		catalog.resident_branches(),
		catalog.resident_leaves(),
		settings.wind.strength_m,
		style.label(),
	);
}
