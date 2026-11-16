//! Debug boss spawn commands ([#1116](https://github.com/ramate-io/maybraid/issues/1116)).
//!
//! Spawns the generated boss encounter via HCSG presentation for visual testing.

use bevy::prelude::*;
use bikanwarda_encounter::BikanwardaBossEncounter;
use lod::hcsg::{HcsgStorage, HcsgRestartRequest};
use player::Player;

/// Request to spawn a boss encounter at a specific location.
#[derive(Component, Clone, Debug)]
pub struct RequestSpawnBoss {
	/// Optional XZ position; defaults to player position.
	pub at: Option<Vec2>,
}

/// System to handle boss spawn requests.
pub fn process_boss_spawn_requests(
	mut commands: Commands,
	requests: Query<(Entity, &RequestSpawnBoss)>,
	storage: Res<HcsgStorage>,
	player: Query<&Transform, With<Player>>,
	mut restart: ResMut<HcsgRestartRequest>,
) {
	for (entity, request) in &requests {
		commands.entity(entity).despawn();
		
		let position = if let Some(xz) = request.at {
			Vec3::new(xz.x, 0.0, xz.y)
		} else if let Ok(transform) = player.single() {
			transform.translation
		} else {
			Vec3::ZERO
		};
		
		info!("Boss spawn requested at {position}");
		
		// Seed the boss encounter in HCSG storage and request presentation
		storage.seed(BikanwardaBossEncounter::chained(), bevy::shape::Aabb3d::new(
			position,
			Vec3::splat(50.0),
		));
		
		// Restart HCSG session to regenerate with new seed
		restart.request();
	}
}

/// Plugin to handle boss spawn debug commands.
pub struct BossSpawnPlugin;

impl Plugin for BossSpawnPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, process_boss_spawn_requests);
	}
}
