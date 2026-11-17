//! Debug boss spawn commands ([#1116](https://github.com/ramate-io/maybraid/issues/1116)).
//!
//! Spawns the generated boss encounter via HCSG presentation for visual testing.

use bevy::prelude::*;
use bikanwarda_encounter::{AuthoredBossSpawns, BikanwardaBossEncounter};
use lod::hcsg::{register_session_seed, universal_bounds, HcsgStorage};
use lod::gen::Bytes;
use player::Player;
use std::sync::Arc;

/// Request to spawn a boss encounter at a specific location.
#[derive(Component, Clone, Debug)]
pub struct RequestSpawnBoss {
	/// Optional XZ position; defaults to player position.
	pub at: Option<Vec2>,
}

/// Persistent boss spawn configuration that survives HCSG session restarts.
#[derive(Resource, Clone, Debug)]
pub struct BossSpawnConfig {
	/// World-space XZ position where the boss should spawn.
	pub position: Vec3,
	/// Whether a boss spawn is active.
	pub active: bool,
}

impl Default for BossSpawnConfig {
	fn default() -> Self {
		Self {
			position: Vec3::ZERO,
			active: false,
		}
	}
}

/// Seeds the boss encounter during HCSG session restarts.
fn seed_boss_encounter(config: Res<BossSpawnConfig>, storage: Res<HcsgStorage>) {
	let position = if config.active {
		Some(config.position)
	} else {
		None
	};
	
	let spawns = AuthoredBossSpawns { position };
	storage.seed(spawns, universal_bounds());
	
	if let Some(pos) = position {
		info!("[BOSS] Reseeding encounter at ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z);
		
		let encounter = BikanwardaBossEncounter::chained_at(pos);
		let id = lod::gen::Id::Bytes(Bytes(
			bikanwarda_encounter::chained_encounter_id().as_bytes()
		));
		
		storage.publish(id, Arc::new(encounter.clone()), encounter.bounds);
	}
}

/// System to handle boss spawn requests.
pub fn process_boss_spawn_requests(
	mut commands: Commands,
	requests: Query<(Entity, &RequestSpawnBoss)>,
	storage: Res<HcsgStorage>,
	player: Query<&Transform, With<Player>>,
	mut config: ResMut<BossSpawnConfig>,
) {
	for (entity, request) in &requests {
		commands.entity(entity).despawn();
		
		let position = if let Some(xz) = request.at {
			Vec3::new(xz.x, 0.0, xz.y)
		} else if let Ok(transform) = player.single() {
			let forward = transform.forward();
			let offset = Vec3::new(forward.x * 8.0, 0.0, forward.z * 8.0);
			transform.translation + offset
		} else {
			Vec3::ZERO
		};
		
		info!("[BOSS] Spawning at world position ({:.1}, {:.1}, {:.1})", position.x, position.y, position.z);
		
		config.position = position;
		config.active = true;
		
		let spawns = AuthoredBossSpawns { position: Some(position) };
		storage.seed(spawns, universal_bounds());
		
		let encounter = BikanwardaBossEncounter::chained_at(position);
		let id = lod::gen::Id::Bytes(Bytes(
			bikanwarda_encounter::chained_encounter_id().as_bytes()
		));
		
		storage.publish(id, Arc::new(encounter.clone()), encounter.bounds);
	}
}

/// Plugin to handle boss spawn debug commands.
pub struct BossSpawnPlugin;

impl Plugin for BossSpawnPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BossSpawnConfig>()
			.add_systems(Update, process_boss_spawn_requests);
		register_session_seed(app, seed_boss_encounter);
	}
}
