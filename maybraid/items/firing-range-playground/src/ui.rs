use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use firearms::WeaponsArmed;
use player::{Npc, Player};

use crate::damage::Health;
use crate::engagement::NpcEngagement;
use crate::session::{RangeMode, RangeSession};

pub(crate) fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Firing range - WASD move - mouse look - R3 POV - RMB / LT focus - click / RT fire"
			.into(),
		empty_console_text:
			"Console: `pause`, `resume`, `free-for-all`, `duel`, `test-dummy`, `help`".into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "help — pause — resume — free-for-all — duel — test-dummy — Enter — history"
			.into(),
	}
}

pub(crate) fn sync_command_status_text(
	armed: Res<WeaponsArmed>,
	engagement: Res<NpcEngagement>,
	session: Res<RangeSession>,
	players: Query<&Health, With<Player>>,
	npcs: Query<&Health, With<Npc>>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let fire = if armed.0 { "armed" } else { "paused" };
	let player_health = players.single().ok().map_or_else(|| "--".into(), health_text);
	let npc_n = npcs.iter().count();
	let npc_health = match session.mode {
		RangeMode::FreeForAll => format!("{npc_n}/{}", session.npc_count),
		RangeMode::AssaultFreeForAll => {
			format!("{npc_n}/{}", session.npc_count + session.civilian_count)
		}
		RangeMode::Duel | RangeMode::TestDummy => {
			npcs.single().ok().map_or_else(|| "--".into(), health_text)
		}
	};
	let response = if session.is_test_dummy() {
		"practice"
	} else if engagement.is_live() {
		"engaged"
	} else {
		"waiting for player shot"
	};
	let mode = match session.mode {
		RangeMode::Duel => "duel",
		RangeMode::FreeForAll => "ffa",
		RangeMode::AssaultFreeForAll => "affa",
		RangeMode::TestDummy => "dummy",
	};
	status.0 =
		format!("{mode} | {fire} | {response} | health: player {player_health} · npc {npc_health}");
}

fn health_text(health: &Health) -> String {
	if health.current <= 0.0 {
		"down".into()
	} else {
		format!("{:.0}/{:.0}", health.current, health.max)
	}
}
