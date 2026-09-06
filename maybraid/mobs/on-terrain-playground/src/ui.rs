use bevy::prelude::*;
use durham_terrain_models::TerrainCellLayout;
use durham_terrain_models::TerrainEntryStore;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};
use mob_intelligence::MemberOf;
use routing_intelligence::RoutingIntelligenceUser;

use crate::mobs::PlaygroundState;
use crate::playground_player::PlaygroundMode;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Mob on terrain — / for commands — Y or F1 drawer".into(),
		empty_console_text:
			"Console: `herd`, `pack`, `both`, `hars`, `ylter`, `hars-ylter`, `rebuild`, `mode character`, `help`"
				.into(),
		root_background: Color::srgba(0.08, 0.16, 0.22, 0.82),
		controls_hint:
			"herd | pack | both | hars | ylter | hars-ylter — rebuild — mode free|character — fly WASD Space/Shift"
				.into(),
	}
}

pub(crate) fn sync_command_status_text(
	mode: Res<PlaygroundMode>,
	state: Res<PlaygroundState>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	hosts: Query<(Entity, &maybraid_mobs::MobScene, &Transform, Option<&RoutingIntelligenceUser>)>,
	plants: Query<(&MemberOf, &GlobalTransform)>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let mode_label = match *mode {
		PlaygroundMode::Free => "free",
		PlaygroundMode::Character => "character",
	};
	let mut lines = vec![format!(
		"mode={mode_label}  cast={}  pois={}  mobs={}",
		state.cast.label(),
		if state.pois_ready { "ok" } else { "wait" },
		if state.mobs_ready { "ok" } else { "wait" }
	)];
	for (host, scene, transform, routing) in &hosts {
		let at = transform.translation;
		let terrain = store
			.composed_height_at(&layout, at.x, at.z)
			.map(|y| format!("{y:.1}"))
			.unwrap_or_else(|| "—".into());
		let hop = routing.and_then(|user| user.current_hop(at));
		let dest = routing.and_then(|user| user.destination);
		let hop_y = hop.map(|p| format!("{:.1}", p.y)).unwrap_or_else(|| "—".into());
		let dest_y = dest.map(|p| format!("{:.1}", p.y)).unwrap_or_else(|| "—".into());
		let member_ys: Vec<_> = plants
			.iter()
			.filter(|(member, _)| member.mob == host)
			.map(|(_, plant)| plant.translation().y)
			.collect();
		let plant_line = plant_summary(&member_ys, at.y);
		lines.push(format!(
			"{:?} host_y={:.1} terrain_y={terrain} hop_y={hop_y} dest_y={dest_y}  {plant_line}",
			scene.mob.kind, at.y
		));
	}
	if hosts.is_empty() {
		lines.push("no hosts — waiting for composed height + terrain collider".into());
	}
	status.0 = lines.join("\n");
}

fn plant_summary(ys: &[f32], host_y: f32) -> String {
	if ys.is_empty() {
		return "plants=0".into();
	}
	let mut min = f32::INFINITY;
	let mut max = f32::NEG_INFINITY;
	for y in ys {
		min = min.min(*y);
		max = max.max(*y);
	}
	format!("plants={} y={:.1}..{:.1} vs_host={:.1}", ys.len(), min, max, max - host_y)
}

pub(crate) fn write_status(
	status: &mut Option<ResMut<GameCommandStatusText>>,
	text: impl Into<String>,
) {
	if let Some(status) = status {
		status.0 = text.into();
	}
}
