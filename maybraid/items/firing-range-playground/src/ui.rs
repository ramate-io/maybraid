use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use firearms::WeaponsArmed;
use player::{Npc, Player};

use crate::damage::Health;
use crate::engagement::NpcEngagement;

pub(crate) fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Firing range - WASD move - mouse look - R3 POV - RMB / LT focus - click / RT fire"
			.into(),
		empty_console_text: "Console: `pause`, `resume`, `help`".into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "help — pause — resume — Enter — history".into(),
	}
}

pub(crate) fn sync_command_status_text(
	armed: Res<WeaponsArmed>,
	engagement: Res<NpcEngagement>,
	players: Query<(Entity, &Health), With<Player>>,
	npcs: Query<(Entity, &Health), With<Npc>>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let fire = if armed.0 { "armed" } else { "paused" };
	let player = players.single().ok();
	let npc = npcs.single().ok();
	let player_health = player.map_or("--".into(), |(_, health)| health_text(health));
	let npc_health = npc.map_or("--".into(), |(_, health)| health_text(health));
	let response = match (player, npc) {
		(Some((player, _)), Some((npc, _))) if engagement.is_provoked(player, npc) => "engaged",
		_ => "waiting for player shot",
	};
	status.0 = format!(
		"player bolt | {fire} | npc {response} | health: player {player_health} · npc {npc_health}"
	);
}

fn health_text(health: &Health) -> String {
	if health.current <= 0.0 {
		"down".into()
	} else {
		format!("{:.0}/{:.0}", health.current, health.max)
	}
}
