use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::show::LastVfxShow;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "VFX — / cmd — F1 drawer — WASD/Space/Shift — mouse look".into(),
		empty_console_text:
			"Console: `god-beam`, `sonic-wave`, `fiery-explosion`, `show`, `cancel`, `hud`, `help`"
				.into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "F1 hide drawer — god-beam — sonic-wave — show — Enter".into(),
	}
}

pub(crate) fn sync_command_status_text(
	last: Res<LastVfxShow>,
	mut status: ResMut<GameCommandStatusText>,
) {
	if last.label.is_empty() {
		if status.0.is_empty() {
			status.0 = "vfx: `fiery-explosion`".into();
		}
		return;
	}
	status.0 = format!("last {}", last.label);
}
