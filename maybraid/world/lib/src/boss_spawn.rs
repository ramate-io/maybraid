//! Debug boss spawn commands ([#1116](https://github.com/ramate-io/maybraid/issues/1116)).
//!
//! Placeholder implementation until #1117 world placement and presentation integration.

use bevy::prelude::*;

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
) {
	for (entity, _request) in &requests {
		commands.entity(entity).despawn();
		// Placeholder: actual spawning will be integrated with HCSG presentation (#1117)
		info!("Boss spawn requested - awaiting HCSG presentation integration");
	}
}

/// Plugin to handle boss spawn debug commands.
pub struct BossSpawnPlugin;

impl Plugin for BossSpawnPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, process_boss_spawn_requests);
	}
}
