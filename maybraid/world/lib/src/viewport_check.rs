//! Viewport visibility check for boss and Lubikan entities (debug-commands feature only).

use bevy::prelude::*;
use bevy::render::camera::Camera;
use player::Player as MaybraidPlayer;
use world_player::Player as WorldPlayer;

/// Log entity positions and viewport visibility before capture.
pub fn log_boss_lubikan_viewport(
	camera_query: Query<(&Camera, &GlobalTransform), With<MaybraidPlayer>>,
	boss_query: Query<(Entity, &GlobalTransform), (With<crate::boss_spawn::BossMarker>, Without<MaybraidPlayer>)>,
	lubikan_query: Query<(Entity, &GlobalTransform), (With<crate::LubikanMarker>, Without<MaybraidPlayer>)>,
	player_query: Query<&GlobalTransform, (With<MaybraidPlayer>, With<WorldPlayer>)>,
) {
	let Ok((camera, camera_transform)) = camera_query.get_single() else {
		return;
	};

	if let Ok(player_transform) = player_query.get_single() {
		let player_pos = player_transform.translation();
		info!("Player position: {:.1}, {:.1}, {:.1}", player_pos.x, player_pos.y, player_pos.z);
	}

	for (entity, transform) in &boss_query {
		let world_pos = transform.translation();
		info!("Boss {:?} world position: {:.1}, {:.1}, {:.1}", entity, world_pos.x, world_pos.y, world_pos.z);
		
		if let Some(viewport_pos) = camera.world_to_viewport(camera_transform, world_pos) {
			info!("Boss viewport position: {:.1}, {:.1} (inside viewport)", viewport_pos.x, viewport_pos.y);
		} else {
			warn!("Boss NOT in viewport");
		}
	}

	for (entity, transform) in &lubikan_query {
		let world_pos = transform.translation();
		info!("Lubikan {:?} world position: {:.1}, {:.1}, {:.1}", entity, world_pos.x, world_pos.y, world_pos.z);
		
		if let Some(viewport_pos) = camera.world_to_viewport(camera_transform, world_pos) {
			info!("Lubikan viewport position: {:.1}, {:.1} (inside viewport)", viewport_pos.x, viewport_pos.y);
		} else {
			warn!("Lubikan NOT in viewport");
		}
	}
}

/// Marker for boss entities to track in viewport.
#[derive(Component)]
pub struct BossMarker;

/// Marker for Lubikan entities to track in viewport.
#[derive(Component)]
pub struct LubikanMarker;
