use bevy::prelude::*;
use game_commands::command::{CommandConsoleOutput, TextEntryFocus, TypedCommandLine};
use game_commands::ui::{GameCommandUiConfig, HudConsoleBlock, HudConsoleViewport, HudStatusLine};

use crate::preview::PreviewConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title:
			"SDF playground - Tab/1-6 - +/- res - / cmd - WASD - up/down history - PgUp/PgDn scroll"
				.into(),
		empty_console_text: "Console: (errors & `help` output) - wheel or PgUp/PgDn".into(),
		root_background: Color::hsla(201.0, 0.69, 0.62, 0.82),
	}
}

fn panel_status(
	config: &PreviewConfig,
	line: &TypedCommandLine,
	text_focus: &TextEntryFocus,
) -> String {
	format!(
		"{} ({})  res_2={}  |  [/] {}  |  buf: {}",
		config.primitive,
		config.primitive.variant_key(),
		config.res_2,
		if text_focus.0 { "cmd ON" } else { "cmd off" },
		if line.0.is_empty() { "_".into() } else { line.0.clone() },
	)
}

pub(crate) fn update_debug_ui(
	camera_query: Query<&Transform, With<Camera3d>>,
	mut hud_text: ParamSet<(
		Query<&mut Text, With<HudStatusLine>>,
		Query<&mut Text, With<HudConsoleBlock>>,
	)>,
	mut console_scroll: Query<&mut ScrollPosition, With<HudConsoleViewport>>,
	config: Res<PreviewConfig>,
	typed: Res<TypedCommandLine>,
	text_focus: Res<TextEntryFocus>,
	console: Res<CommandConsoleOutput>,
) {
	let Ok(transform) = camera_query.single() else {
		return;
	};
	let pos = transform.translation;

	if console.is_changed() {
		for mut sp in &mut console_scroll {
			sp.0 = Vec2::ZERO;
		}
	}

	if let Ok(mut status) = hud_text.p0().single_mut() {
		status.0 = format!(
			"{}\nCam {:.1}, {:.1}, {:.1}   ·   help · Enter · ↑↓ hist · PgUp/PgDn · ⇧↑↓",
			panel_status(&config, &typed, &text_focus),
			pos.x,
			pos.y,
			pos.z
		);
	}
	if let Ok(mut block) = hud_text.p1().single_mut() {
		block.0 = if console.0.is_empty() {
			"Console: (errors & `help` output) — wheel or PgUp/PgDn".into()
		} else {
			console.0.clone()
		};
	}
}
