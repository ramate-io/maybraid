//! Higher-order NPC vantage: refresh the watched point and request a replan.
//!
//! Movement intelligence does not lock onto the player. This playground decides
//! when the [`MovementObjective`] location is stale.

use bevy::prelude::*;
use movement_intelligence::{
	MovementIntelligence, MovementLocation, MovementObjective, ReplanMovement,
};
use player::{Npc, Player};

const REFRESH_INTERVAL: f32 = 0.7;
const MOVE_THRESHOLD: f32 = 1.35;
const WATCH_RADIUS: f32 = 1.4;
const HIDE_WEIGHT: f32 = 10.0;
const SIGHTLINE_WEIGHT: f32 = 14.0;

#[derive(Resource, Debug)]
pub(crate) struct NpcVantageRefresh {
	elapsed: f32,
	last_point: Option<Vec3>,
}

impl Default for NpcVantageRefresh {
	fn default() -> Self {
		Self { elapsed: REFRESH_INTERVAL, last_point: None }
	}
}

pub(crate) fn vantage_on_player(point: Vec3) -> MovementObjective {
	MovementObjective::VantageOn {
		location: MovementLocation::new(point, WATCH_RADIUS),
		hide_weight: HIDE_WEIGHT,
		sightline_weight: SIGHTLINE_WEIGHT,
	}
}

pub(crate) fn refresh_npc_vantage(
	time: Res<Time>,
	mut refresh: ResMut<NpcVantageRefresh>,
	players: Query<&Transform, With<Player>>,
	mut npcs: Query<(Entity, &mut MovementIntelligence), With<Npc>>,
	mut commands: Commands,
) {
	let Ok(player) = players.single() else {
		return;
	};
	refresh.elapsed += time.delta_secs();
	if refresh.elapsed < REFRESH_INTERVAL {
		return;
	}
	let point = player.translation;
	let moved = refresh
		.last_point
		.map(|last| {
			Vec2::new(last.x, last.z).distance(Vec2::new(point.x, point.z)) >= MOVE_THRESHOLD
				|| (last.y - point.y).abs() >= MOVE_THRESHOLD
		})
		.unwrap_or(true);
	if !moved {
		return;
	}
	refresh.elapsed = 0.0;
	refresh.last_point = Some(point);
	for (entity, mut brain) in &mut npcs {
		brain.objective = vantage_on_player(point);
		commands.entity(entity).insert(ReplanMovement);
	}
}
