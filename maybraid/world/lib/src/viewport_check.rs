//! Viewport visibility check for boss and Lubikan entities (debug-commands feature only).

use bevy::prelude::*;
use player::Player as MaybraidPlayer;
use world_player::Player as WorldPlayer;

/// Log entity positions (viewport check stub - full implementation deferred).
pub fn log_boss_lubikan_positions(
	boss_query: Query<(Entity, &GlobalTransform), With<BossMarker>>,
	lubikan_query: Query<(Entity, &GlobalTransform), With<LubikanMarker>>,
	player_query: Query<&GlobalTransform, (With<MaybraidPlayer>, With<WorldPlayer>)>,
) {
	if let Ok(player_transform) = player_query.single() {
		let player_pos = player_transform.translation();
		info!("[CAPTURE] Player position: ({:.1}, {:.1}, {:.1})", player_pos.x, player_pos.y, player_pos.z);
	}

	for (entity, transform) in &boss_query {
		let world_pos = transform.translation();
		info!("[CAPTURE] Boss {:?} position: ({:.1}, {:.1}, {:.1})", entity, world_pos.x, world_pos.y, world_pos.z);
	}

	for (entity, transform) in &lubikan_query {
		let world_pos = transform.translation();
		info!("[CAPTURE] Lubikan {:?} position: ({:.1}, {:.1}, {:.1})", entity, world_pos.x, world_pos.y, world_pos.z);
	}
}

/// Marker for boss entities to track in viewport.
#[derive(Component)]
pub struct BossMarker;

/// Marker for Lubikan entities to track in viewport.
#[derive(Component)]
pub struct LubikanMarker;
