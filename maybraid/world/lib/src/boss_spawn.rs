//! Debug boss spawn commands ([#1116](https://github.com/ramate-io/maybraid/issues/1116)).
//!
//! Spawns the generated boss encounter via HCSG presentation for visual testing.
//!
//! ## Spawn Policy
//!
//! Fixed world anchor: the boss spawns at an authored XZ position with terrain-fit
//! ground placement. The position is preserved exactly across:
//! - Full world restart (via persistent BossSpawnConfig resource)
//!
//! Ground fit waits for published terrain/pad collider acceptance before finalizing
//! the boss Y position. This matches the player spawn's terrain readiness contract:
//! `surface_or_hold_at` returns a search hint only; the spawn system waits for
//! `terrain_height_at` to return Some(height) before publishing the boss.
//!
//! The boss does NOT follow player respawns; it stays at its single authored anchor.
//!
//! **Note:** Region departure and re-entry persistence (via HCSG cache eviction/rediscovery)
//! depends on work in progress on the parent branch (#1601). This PR addresses ground-fit,
//! spawn policy, and within-session restart only.

use bevy::prelude::*;
use bikanwarda_encounter::{AuthoredBossSpawns, BikanwardaBossEncounter};
use lod::hcsg::{register_session_seed, universal_bounds, HcsgStorage};
use lod::gen::Bytes;
use player::Player;
use std::sync::Arc;

use crate::terrain_query::query_published_terrain_height;

/// Request to spawn a boss encounter at a specific location.
#[derive(Component, Clone, Debug)]
pub struct RequestSpawnBoss {
	/// Optional XZ position; defaults to player position.
	pub at: Option<Vec2>,
}

/// Pending boss spawn waiting for terrain readiness.
/// 
/// The boss spawn uses the same pending/readiness pattern as player placement:
/// it waits for published terrain/pad height before finalizing the Y coordinate.
#[derive(Component, Clone, Debug)]
pub struct PendingBossSpawn {
	/// Target XZ position in world space.
	pub xz: Vec2,
	/// How many updates we've waited for terrain.
	pub wait_ticks: u32,
}

/// Persistent boss spawn configuration that survives HCSG session restarts.
/// 
/// Note: This preserves position across within-session HCSG restarts only.
/// Cross-session persistence (region departure/re-entry) depends on HCSG
/// cache eviction/rediscovery work on #1601.
#[derive(Resource, Clone, Debug)]
pub struct BossSpawnConfig {
	/// World-space position where the boss should spawn.
	/// Only valid when active is true.
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

/// Seeds the boss encounter session root during HCSG session restarts.
///
/// Only seeds `AuthoredBossSpawns` (the session root), not the boss encounter itself.
/// The boss encounter is derived data that will be regenerated on-demand when presentation
/// requests it via `original_ids_for` and `build_with_id`.
///
/// The position in `BossSpawnConfig` already includes terrain-fit from the original spawn,
/// so it's preserved exactly across session restart boundaries and region departure/re-entry.
fn seed_boss_encounter(config: Res<BossSpawnConfig>, storage: Res<HcsgStorage>) {
	let position = if config.active {
		Some(config.position)
	} else {
		None
	};
	
	let spawns = AuthoredBossSpawns { position };
	storage.seed(spawns, universal_bounds());
	
	if position.is_some() {
		info!("[BOSS] Reseeded AuthoredBossSpawns at session restart");
	}
}

/// Get the boss hull spawn height offset for ground clearance.
///
/// Derives the offset from the actual Bikanwarda encounter boss recipe,
/// ensuring the value stays synchronized with hull changes.
fn boss_spawn_height_offset() -> f32 {
	use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaSpec};
	use characters::CharacterRecipe;
	
	let spec = BikanwardaSpec::encounter();
	let recipe = BikanwardaActorRecipe::from_spec(spec);
	let hull = recipe.components().scaled_locomotion_capsule();
	hull.spawn_height()
}

/// System to handle boss spawn requests.
/// 
/// Consumes RequestSpawnBoss and creates PendingBossSpawn, which waits for
/// published terrain before finalizing the Y coordinate.
pub fn process_boss_spawn_requests(
	mut commands: Commands,
	requests: Query<(Entity, &RequestSpawnBoss)>,
	player: Query<&Transform, With<Player>>,
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
		
		info!(
			"[BOSS] Spawning boss at XZ ({:.1}, {:.1}), waiting for published terrain",
			xz.x, xz.y
		);
		
		// Create pending spawn that waits for terrain readiness
		commands.spawn(PendingBossSpawn { xz, wait_ticks: 0 });
	}
}

/// Poll pending boss spawns and finalize when terrain is ready.
/// 
/// This system implements the terrain readiness contract: wait for
/// published terrain height instead of consuming the holding elevation as final position.
/// Durham resources are queried optionally; if missing, bosses remain pending.
pub fn poll_pending_boss_spawns(
	mut commands: Commands,
	mut pending: Query<(Entity, &mut PendingBossSpawn)>,
	surface: Option<durham::DurhamSurface>,
	storage: Option<Res<HcsgStorage>>,
	mut config: ResMut<BossSpawnConfig>,
) {
	for (entity, mut pending_spawn) in &mut pending {
		pending_spawn.wait_ticks += 1;
		
		// Try to get published terrain height (terrain/pad with collider acceptance)
		// If Durham resources are missing, boss remains pending until they're available
		let terrain_height = query_published_terrain_height(
			surface.as_ref(),
			storage.as_deref(),
			pending_spawn.xz,
		);
		
		if let Some(terrain_y) = terrain_height {
			// Terrain is ready! Apply boss hull spawn_height offset for ground fit
			let spawn_height_offset = boss_spawn_height_offset();
			let position = Vec3::new(
				pending_spawn.xz.x,
				terrain_y + spawn_height_offset,
				pending_spawn.xz.y,
			);
			
			info!(
				"[BOSS] Terrain ready after {} ticks. Finalizing boss at ({:.1}, {:.1}, {:.1}) [terrain_y={:.1}, hull_offset={:.1}]",
				pending_spawn.wait_ticks,
				position.x, position.y, position.z,
				terrain_y,
				spawn_height_offset
			);
			
			config.position = position;
			config.active = true;
			
			// Seed AuthoredBossSpawns (session root) with terrain-fit position
			let spawns = AuthoredBossSpawns { position: Some(position) };
			storage.as_ref().unwrap().seed(spawns, universal_bounds());
			
			// Also publish the derived encounter for immediate presentation
			let encounter = BikanwardaBossEncounter::chained_at(position);
			let id = lod::gen::Id::Bytes(Bytes(
				bikanwarda_encounter::chained_encounter_id().as_bytes()
			));
			storage.as_ref().unwrap().publish(id, Arc::new(encounter.clone()), encounter.bounds);
			
			commands.entity(entity).despawn();
		} else if pending_spawn.wait_ticks % 60 == 0 {
			// Log waiting status every 60 ticks (~1 second at 60 FPS)
			info!(
				"[BOSS] Waiting for published terrain at XZ ({:.1}, {:.1})... {} ticks",
				pending_spawn.xz.x, pending_spawn.xz.y, pending_spawn.wait_ticks
			);
		}
	}
}

/// Plugin to handle boss spawn debug commands.
pub struct BossSpawnPlugin;

impl Plugin for BossSpawnPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BossSpawnConfig>()
			.add_systems(
				Update,
				(
					process_boss_spawn_requests,
					poll_pending_boss_spawns,
				).chain(),
			);
		register_session_seed(app, seed_boss_encounter);
	}
}
