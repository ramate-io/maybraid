//! Mob spawn materialization: placed hosts must produce live bodies with brains.
//!
//! Requires `test-support` feature for `RequestSpawnMobs`.

use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, Virtual};
use lod::LodViewer;
use lod::scene::level::LodSceneLevel;
use lod::scene::host::LodLevelRoot;
use mob_intelligence::MemberOf;
use mob_scenes::{MobKind, MobScene};
use player::{Npc, Player};
use std::time::Duration;

use maybraid_world::{RequestSpawnMobs, WorldPlugin};

const DT: f32 = 0.016; // ~60 FPS to match game timing

fn full_game_app() -> App {
	let mut app = App::new();
	
	// Use the same plugin setup as the shipped game, but skip render-only plugins
	app.add_plugins(MinimalPlugins)
		.add_plugins(bevy::asset::AssetPlugin::default())
		.add_plugins(bevy::scene::ScenePlugin)
		.add_plugins(bevy::state::app::StatesPlugin)
		.init_asset::<Mesh>()
		.init_asset::<StandardMaterial>()
		.init_asset::<bevy::world_serialization::WorldAsset>()
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
		.insert_resource(maybraid_virtual_pad_controller::PadHidPolicy::External);
	
	// Allow errors for systems that expect render app or missing player stack
	app.set_error_handler(bevy::ecs::error::warn);
	
	// Add core world plugins needed for mob spawning (from WorldPlugin but skipping render-dependent ones)
	// Skip: VfxPlugin (has HanabiPlugin which needs RenderApp)
	// Skip: render_scale, screenshot (need RenderApp)
	// Skip: WorldCombatHud, camera plugins, ragdoll, presentation (render-dependent)
	
	app.add_plugins(maybraid_world::WorldMaterialRefPlugin);
	app.add_plugins(maybraid_world::WorldLayersPlugin); // Includes mob systems
	app.add_plugins(maybraid_world::WorldIntelligencePlugin); // Mob AI
	
	// Add mob spawn system (normally added by WorldPlugin)
	app.add_systems(bevy::app::Update, maybraid_world::apply_spawn_mob_requests);
	
	// Add essential resources that WorldPlugin sets up
	app.insert_resource(lod::Bullseye { inner: 50.0, outer: 2000.0 });
	app.insert_resource(lod::OpenLattice {
		exclude_extent: 2000.0,
		outer_extent: 8000.0,
		tile_size: 500.0,
	});
	
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(30));
	
	app.finish();
	app.cleanup();
	
	app
}

#[derive(Debug)]
struct HostDiagnostics {
	entity: Entity,
	center: Vec3,
	transform: Option<Vec3>,
	global_transform: Option<Vec3>,
	lod_level: Option<LodSceneLevel>,
	high_root_exists: bool,
	member_count: usize,
	npc_count: usize,
}

fn collect_host_diagnostics(world: &mut World) -> Vec<HostDiagnostics> {
	let mut diagnostics = Vec::new();
	
	// Collect all hosts
	let mut hosts_query = world.query::<(Entity, &MobScene, Option<&Transform>, Option<&GlobalTransform>, Option<&LodSceneLevel>)>();
	let hosts: Vec<_> = hosts_query.iter(world).map(|(e, s, t, g, l)| {
		(e, s.clone(), t.copied(), g.copied(), l.copied())
	}).collect();
	
	for (entity, scene, transform, global_transform, lod_level) in hosts {
		// Check if High root exists  
		let high_root_exists = if let Some(children) = world.get::<Children>(entity) {
			let mut exists = false;
			for child in children.iter() {
				if let Some(root) = world.get::<LodLevelRoot>(child) {
					if root.0 == LodSceneLevel::High {
						exists = true;
						break;
					}
				}
			}
			exists
		} else {
			false
		};
		
		// Count members and NPCs belonging to this host
		let member_count = if let Some(children) = world.get::<Children>(entity) {
			let mut count = 0;
			for child in children.iter() {
				if world.get::<MemberOf>(child).is_some() {
					count += 1;
				}
			}
			count
		} else {
			0
		};
		
		let npc_count = if let Some(children) = world.get::<Children>(entity) {
			let mut count = 0;
			for child in children.iter() {
				if world.get::<Npc>(child).is_some() {
					count += 1;
				}
			}
			count
		} else {
			0
		};
		
		diagnostics.push(HostDiagnostics {
			entity,
			center: scene.center,
			transform: transform.map(|t| t.translation),
			global_transform: global_transform.map(|g| g.translation()),
			lod_level,
			high_root_exists,
			member_count,
			npc_count,
		});
	}
	
	diagnostics
}

#[test]
fn placed_raider_spawns_body_and_brain_in_full_game() -> anyhow::Result<()> {
	let mut app = full_game_app();
	
	// Spawn a real Player entity at a position matching the evidence
	// The reviewer had the player grounded and supported
	let player_pos = Vec3::new(1746.8, 0.0, 1025.9);
	let _player = app.world_mut().spawn((
		Player,
		LodViewer,
		Transform::from_translation(player_pos),
		GlobalTransform::from(Transform::from_translation(player_pos)),
	)).id();
	
	// Warm up: step frames to let terrain/streaming/LOD systems initialize
	// The reviewer noted 34 existing hosts before spawning, so the world was already running
	eprintln!("Warming up world systems...");
	for frame in 0..300 {
		app.update();
		if frame % 100 == 0 {
			let host_count = app.world_mut().query::<&MobScene>().iter(app.world()).count();
			eprintln!("Frame {}: {} existing hosts", frame, host_count);
		}
	}
	
	// Record initial host count (should have some from world streaming)
	let initial_hosts = collect_host_diagnostics(app.world_mut());
	let initial_count = initial_hosts.len();
	eprintln!("Initial host count: {}", initial_count);
	
	// Spawn one Raider at the exact position from the evidence
	// "mobs spawn raider 1 --at 1746.8,1025.9 --spacing 8 --seed 1"
	let spawn_pos = Vec2::new(1746.8, 1025.9);
	app.world_mut().spawn(RequestSpawnMobs {
		kind: MobKind::Raider,
		count: 1,
		at: Some(spawn_pos),
		spacing: 8.0,
		seed: 1,
	});
	
	app.update();
	
	// Check that a new host was created
	let hosts_after_spawn = collect_host_diagnostics(app.world_mut());
	eprintln!("Hosts after spawn request: {}", hosts_after_spawn.len());
	anyhow::ensure!(
		hosts_after_spawn.len() == initial_count + 1,
		"Expected {} hosts after spawn, found {}. New host may not have been created!",
		initial_count + 1,
		hosts_after_spawn.len()
	);
	
	// Find the new host
	let new_host = hosts_after_spawn.iter()
		.find(|h| !initial_hosts.iter().any(|ih| ih.entity == h.entity))
		.ok_or_else(|| anyhow::anyhow!("Could not find new host"))?;
	
	eprintln!("New host immediately after spawn: {:#?}", new_host);
	
	// Verify the host has correct center, transform, and GlobalTransform
	let expected_spawn_xz = Vec2::new(1746.8, 1025.9);
	anyhow::ensure!(
		new_host.center.xz().distance(expected_spawn_xz) < 10.0,
		"Host center should be near spawn position {:?}, got {:?}",
		expected_spawn_xz,
		new_host.center
	);
	
	if let Some(transform) = new_host.transform {
		anyhow::ensure!(
			transform.xz().distance(expected_spawn_xz) < 10.0,
			"Host Transform should be near spawn position {:?}, got {:?}",
			expected_spawn_xz,
			transform
		);
	} else {
		eprintln!("WARNING: Host missing Transform component immediately after spawn");
	}
	
	if let Some(global_transform) = new_host.global_transform {
		anyhow::ensure!(
			global_transform.xz().distance(expected_spawn_xz) < 10.0,
			"Host GlobalTransform should be near spawn position {:?}, got {:?}",
			expected_spawn_xz,
			global_transform
		);
	} else {
		eprintln!("WARNING: Host missing GlobalTransform component - THIS IS LIKELY THE BUG");
	}
	
	// Step through ~40 seconds of frames to match the evidence timeline
	// At 60 FPS (0.016s per frame), 40s = 2500 frames
	eprintln!("Stepping through ~40 seconds of simulation...");
	for frame in 0..2500 {
		app.update();
		
		// Check host state every 5 seconds (every ~300 frames)
		if frame % 300 == 0 {
			let current_hosts = collect_host_diagnostics(app.world_mut());
			if let Some(host) = current_hosts.iter().find(|h| h.entity == new_host.entity) {
				let elapsed_secs = frame as f32 * DT;
				eprintln!("t={:.1}s: {:#?}", elapsed_secs, host);
				
				// If we have bodies, we can stop early
				if host.member_count > 0 && host.npc_count > 0 {
					eprintln!("SUCCESS: Bodies materialized at t={:.1}s", elapsed_secs);
					break;
				}
			} else {
				eprintln!("WARNING: Host disappeared at frame {}", frame);
			}
		}
	}
	
	// Final diagnostics
	let final_hosts = collect_host_diagnostics(app.world_mut());
	let final_host = final_hosts.iter()
		.find(|h| h.entity == new_host.entity)
		.ok_or_else(|| anyhow::anyhow!("Host disappeared"))?;
	
	eprintln!("Final host state: {:#?}", final_host);
	
	// Verify the host selected High LOD
	anyhow::ensure!(
		final_host.lod_level == Some(LodSceneLevel::High),
		"Expected High LOD, got {:?}",
		final_host.lod_level
	);
	
	// Verify High root exists
	anyhow::ensure!(
		final_host.high_root_exists,
		"Expected High root to exist"
	);
	
	// Count total mob members and NPCs
	let member_count = app.world_mut().query::<&MemberOf>().iter(app.world()).count();
	let npc_count = app.world_mut().query::<&Npc>().iter(app.world()).count();
	
	anyhow::ensure!(
		member_count > 0,
		"Expected mob members to materialize, found {member_count}"
	);
	
	anyhow::ensure!(
		npc_count > 0,
		"Expected NPC brains to materialize, found {npc_count}"
	);
	
	anyhow::ensure!(
		member_count == npc_count,
		"Expected member_count ({member_count}) == npc_count ({npc_count})"
	);
	
	Ok(())
}
