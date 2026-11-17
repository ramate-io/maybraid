//! Debug boss spawn commands ([#1116](https://github.com/ramate-io/maybraid/issues/1116)).
//!
//! Spawns the generated boss encounter via HCSG presentation for visual testing.
//!
//! ## Spawn Policy
//!
//! Fixed world anchor: the boss spawns at an authored XZ position with terrain-fit
//! ground placement. The position is preserved exactly across:
//! - Region departure and re-entry (via HCSG session restart)
//! - Full world restart (via persistent BossSpawnConfig resource)
//!
//! Ground fit applies the boss hull's `spawn_height()` offset so the boss's feet/hull
//! bottom rest on the terrain at the authored XZ, with the Y sampled from composed
//! Durham terrain. This matches the player spawn policy's terrain-fit behavior.
//!
//! The boss does NOT follow player respawns; it stays at its single authored anchor.

use bevy::prelude::*;
use bikanwarda_character::BikanwardaSpec;
use bikanwarda_encounter::BikanwardaBossEncounter;
use characters::LocomotionCapsule;
use durham::DurhamSurface;
use lod::hcsg::{register_session_seed, HcsgStorage};
use lod::gen::Bytes;
use maybraid_spawns_support::surface_or_hold_at;
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
///
/// Restores the boss at the same authored world position from `BossSpawnConfig`.
/// The position already includes terrain-fit and spawn_height offset from the
/// original spawn, so it's preserved exactly across session boundaries.
fn seed_boss_encounter(config: Res<BossSpawnConfig>, storage: Res<HcsgStorage>) {
	if !config.active {
		return;
	}
	
	info!("[BOSS] Reseeding encounter at ({:.1}, {:.1}, {:.1})", 
		config.position.x, config.position.y, config.position.z);
	
	let encounter = BikanwardaBossEncounter::chained_at(config.position);
	let id = lod::gen::Id::Bytes(Bytes(
		bikanwarda_encounter::chained_encounter_id().as_bytes()
	));
	
	storage.publish(id, Arc::new(encounter.clone()), encounter.bounds);
}

/// System to handle boss spawn requests.
pub fn process_boss_spawn_requests(
	mut commands: Commands,
	requests: Query<(Entity, &RequestSpawnBoss)>,
	storage: Res<HcsgStorage>,
	surface: DurhamSurface,
	player: Query<&Transform, With<Player>>,
	mut config: ResMut<BossSpawnConfig>,
) {
	for (entity, request) in &requests {
		commands.entity(entity).despawn();
		
		// Determine XZ position (either explicit or player-relative)
		let xz = if let Some(xz) = request.at {
			xz
		} else if let Ok(transform) = player.single() {
			let forward = transform.forward();
			let offset = Vec2::new(forward.x * 8.0, forward.z * 8.0);
			transform.translation.xz() + offset
		} else {
			Vec2::ZERO
		};
		
		// Sample terrain height at spawn XZ
		let terrain_y = surface_or_hold_at(&surface, xz);
		
		// Get boss hull and apply spawn_height offset for ground fit
		let boss_spec = BikanwardaSpec::encounter();
		let boss_recipe = bikanwarda_character::BikanwardaActorRecipe::from_spec(boss_spec.clone());
		let boss_hull = boss_recipe.components().scaled_locomotion_capsule();
		let spawn_height_offset = boss_hull.spawn_height();
		
		// Final world position with ground fit
		let position = Vec3::new(xz.x, terrain_y + spawn_height_offset, xz.y);
		
		info!(
			"[BOSS] Spawning at world position ({:.1}, {:.1}, {:.1}) [terrain_y={:.1}, hull_offset={:.1}]",
			position.x, position.y, position.z, terrain_y, spawn_height_offset
		);
		
		config.position = position;
		config.active = true;
		
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
