use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use tether_intelligence::{TetherIntelligenceUser, TetherMemory};

use crate::playground_player::PlaygroundMode;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Routing on Durham — / for commands — Y or F1 drawer".into(),
		empty_console_text: "Console: `tether`, `stalk`, `go <x> <z>`, `mode character`, `help`"
			.into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint:
			"help — tether [r] — stalk [r] — idle|drive — go <x> <z> — mode free|character".into(),
	}
}

pub(crate) fn sync_command_status_text(
	mode: Res<PlaygroundMode>,
	tethers: Query<(&TetherIntelligenceUser, &TetherMemory)>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let mode_label = match *mode {
		PlaygroundMode::Free => "free",
		PlaygroundMode::Character => "character",
	};
	let tether = tethers.iter().next().map(|(user, memory)| {
		let kind = match user.objective {
			tether_intelligence::TetherObjective::Tether(_, r) => format!("tether r={r:.0}"),
			tether_intelligence::TetherObjective::Stalk(_, r) => format!("stalk r={r:.0}"),
		};
		let grant = if user.enabled { "drive" } else { "idle" };
		let done = if memory.satisfied { "ok" } else { "open" };
		format!("{kind} {grant} {done} rem={:.0}", memory.remaining)
	});
	let extra = tether.as_deref().unwrap_or("no tether");
	status.0 = format!("mode={mode_label}  {extra}  orange/yellow/cyan = corridor");
}

pub(crate) fn write_status(
	status: &mut Option<ResMut<GameCommandStatusText>>,
	text: impl Into<String>,
) {
	if let Some(status) = status {
		status.0 = text.into();
	}
}
