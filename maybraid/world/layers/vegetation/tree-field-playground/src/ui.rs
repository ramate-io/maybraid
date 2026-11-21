use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use vegetation_tree_field::{TreeField, TreeFieldSettings};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "GPU tree canopies - / cmd - L look - WASD - Space/Shift up/down".into(),
		empty_console_text: "Console: `spawn grove`, `leaves dither`, `camera orbit`, `help`".into(),
		root_background: Color::srgba(0.10, 0.16, 0.08, 0.82),
		controls_hint: "help — spawn — leaves — camera — wind — lod — shadows — compare".into(),
	}
}

pub(crate) fn sync_command_status_text(
	field: Res<TreeField>,
	settings: Res<TreeFieldSettings>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let style = settings
		.leaf_close_style
		.or_else(|| field.plants.first().map(|plant| plant.leaf_close_style()))
		.unwrap_or_default();
	status.0 = format!(
		"plants {} | wood {} | leaves {} | wind {:.2} m | leaves {}\n\
		 GPU wood+foliage | nested LOD | BotW canopy palette",
		field.plants.len(),
		field.branches.len(),
		field.leaves.len(),
		settings.wind.strength_m,
		style.label(),
	);
}
