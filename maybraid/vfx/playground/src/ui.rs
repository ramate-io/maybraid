use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::show::LastVfxShow;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "VFX — / cmd — WASD/Space/Shift — mouse look".into(),
		empty_console_text:
			"Console: `fiery-explosion`, `show dust`, `orbit`, `pause`, `play`, `reset`, `cancel`, `spread`, `help`".into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "show dust — pause/play — reset/cancel — Enter".into(),
	}
}

pub(crate) fn sync_command_status_text(
	last: Res<LastVfxShow>,
	herds: Query<&maybraid_vfx::parts::herd::HerdState>,
	instances: Query<&maybraid_vfx::VfxInstance>,
	mut status: ResMut<GameCommandStatusText>,
	states: Query<&maybraid_vfx::parts::combined::CombinedState>,
	scene: Res<maybraid_vfx::parts::combined::interaction::InteractionScene>,
) {
	if let Some(h) = herds.iter().next() {
		status.0 = format!(
			"HERD · {} · distance approximation · {} bodies · fine {:?} · rejected {} · / herd-control cartoon|combined|none|off|on · pause/play reset/cancel · inspect close",
			if h.part.settings.group { "regional cloud" } else { "individual trails" },
			h.active(),
			h.fine_counts(),
			h.rejected
		);
		return;
	}
	if last.label.is_empty() {
		if status.0.is_empty() {
			status.0 = "vfx: `fiery-explosion`".into();
		}
		return;
	}
	status.0 = format!(
		"last {} | ages {:?}",
		last.label,
		instances
			.iter()
			.map(|i| (i.seed, (i.age * 1000.).round() / 1000.))
			.collect::<Vec<_>>()
	);
	if let Some(s) = states.iter().find(|s| s.options.motion.interactive()) {
		status.0 += &format!(
			" | {:?} {:?} {:?} | arrows: capsule | interaction / combined | wake {} | wind {:?}",
			scene.scenario, s.options.layers, s.options.quality, scene.wake, scene.wind
		);
	}
}
