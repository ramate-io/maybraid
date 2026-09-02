//! Firing range: pad + Les Halles stack → player + firearm-user plugins.

mod buildings_lod;
pub mod commands;
mod damage;
mod engagement;
mod hud;
mod les_halles;
mod loadout;
mod range;
mod session;
mod ui;
mod vantage;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use buildings_lod::FiringRangeBuildingsLodPlugin;
use crozon_character_items::ItemRng;
use crozon_characters::CharacterHostsPlugin;
use firearm_intelligence::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
use firearm_user::{spawn_reticle, FirearmUserPlugin};
use firearms::{FirearmHostsPlugin, FirearmWeaponSystems, FirearmWeaponsPlugin};
use game_commands::command::{GameCommandPlugin, TextEntryFocus};
use les_halles::LesHallesSpawn;
use lod::LodRefreshSystems;
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_input::{PadGameplayEnabled, VirtualPadSystems};
use movement_intelligence::{
	CandidateBudget, MovementIntelligenceLimits, MovementIntelligencePlugin,
};
use movement_intelligence_richmond::RichmondAvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use player::{
	spawn_npc_with_hidden_capsule, spawn_player_with_hidden_capsule, Npc, Player, PlayerLook,
	PlayerPlugin,
};
use player_camera::{spawn_follow_camera, PlayerCameraPlugin};
use projectiles::tick_flights;
use richmond_building_components::{
	apply_parent_confines, FurnitureWireframePlugin, LabelWireframePlugin,
};
use richmond_building_physics::BuildingWalkColliderPlugin;
use session::{AppliedSession, LoadoutRng, RangeMode, RangeSession};

pub struct FiringRangePlugin;

impl Plugin for FiringRangePlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(MovementIntelligenceLimits {
			max_budget: CandidateBudget { max_candidates: 32, max_steps: 4, horizon: 80.0 },
		})
		.add_plugins(FirearmHostsPlugin)
		.add_plugins(FirearmWeaponsPlugin)
		.add_plugins(CharacterHostsPlugin)
		.add_plugins(CharacterControllerPlugin)
		.add_plugins(PlayerPlugin)
		.add_plugins(PlayerCameraPlugin)
		.add_plugins(FirearmUserPlugin)
		.add_plugins(FiringRangeBuildingsLodPlugin)
		.add_plugins((FurnitureWireframePlugin, LabelWireframePlugin))
		.add_plugins(BuildingWalkColliderPlugin)
		.add_plugins(MovementIntelligencePlugin::<RichmondAvianMovementSurface<'_, '_>>::default())
		.add_plugins(FirearmIntelligencePlugin)
		.add_plugins(MovementRealizationPlugin)
		.init_resource::<LesHallesSpawn>()
		.init_resource::<hud::DamageTicks>()
		.init_resource::<damage::CombatRespawn>()
		.init_resource::<engagement::NpcEngagement>()
		.init_resource::<RangeSession>()
		.init_resource::<AppliedSession>()
		.insert_resource(LoadoutRng(ItemRng::from_entropy()))
		.add_message::<damage::DamageTaken>()
		.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
		.add_systems(
			Startup,
			(
				spawn_follow_camera_system,
				setup_lighting,
				range::setup_range,
				spawn_reticle_system,
				hud::spawn_combat_hud,
			)
				.chain(),
		)
		.add_systems(
			PostStartup,
			(
				les_halles::setup_les_halles,
				session::apply_session,
				spawn_player_system,
				spawn_npc_system,
			)
				.chain(),
		)
		.add_systems(PreUpdate, gate_pad.before(VirtualPadSystems::Produce))
		.add_systems(
			Update,
			(
				session::apply_session,
				session::spawn_player_character,
				session::spawn_npc_character,
				session::spawn_held_system,
				respawn_combatants,
				vantage::assign_combat_targets.before(FirearmIntelligenceSystems::Spotting),
				les_halles::draw_circulation_gizmos,
				apply_parent_confines.after(LodRefreshSystems::Cull),
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				hud::ensure_world_health_bars,
				hud::sync_health_hud,
				hud::sync_world_health_bars,
				hud::update_damage_indicators,
			),
		)
		.add_systems(
			PostUpdate,
			(damage::apply_projectile_damage, hud::ingest_damage_indicators, damage::despawn_dead)
				.chain()
				.after(tick_flights),
		)
		.add_systems(
			PostUpdate,
			engagement::gate_npc_fire
				.after(FirearmIntelligenceSystems::Fire)
				.before(FirearmWeaponSystems::Fire),
		)
		.add_systems(PostUpdate, engagement::record_player_shot.after(FirearmWeaponSystems::Fire));
	}
}

fn spawn_follow_camera_system(mut commands: Commands) {
	spawn_follow_camera(&mut commands);
}

fn spawn_player_system(
	session: Res<RangeSession>,
	mut commands: Commands,
	spawn: Res<LesHallesSpawn>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	if session.epoch != 0 {
		return;
	}
	spawn_player_at(&mut commands, &spawn, &mut meshes, &mut materials);
}

fn spawn_npc_system(
	session: Res<RangeSession>,
	mut commands: Commands,
	spawn: Res<LesHallesSpawn>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	if session.epoch != 0 {
		return;
	}
	spawn_npc_at(&mut commands, &spawn, &mut meshes, &mut materials);
}

pub(crate) fn spawn_player_at(
	commands: &mut Commands,
	spawn: &LesHallesSpawn,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let player = spawn_player_with_hidden_capsule(commands, meshes, materials);
	commands.entity(player).insert((
		Transform::from_translation(spawn.player),
		PlayerLook { yaw: spawn.look_yaw, ..default() },
		damage::Health::default(),
	));
}

pub(crate) fn spawn_npc_at(
	commands: &mut Commands,
	spawn: &LesHallesSpawn,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let npc = spawn_npc_with_hidden_capsule(
		commands,
		spawn.npc,
		PlayerLook { yaw: spawn.look_yaw, ..default() },
		meshes,
		materials,
	);
	session::install_npc_combat(commands, npc, spawn.npc, None);
}

#[allow(clippy::too_many_arguments)]
fn respawn_combatants(
	time: Res<Time>,
	spawn: Res<LesHallesSpawn>,
	session: Res<RangeSession>,
	players: Query<(), With<Player>>,
	npcs: Query<(), With<Npc>>,
	mut respawn: ResMut<damage::CombatRespawn>,
	mut rng: ResMut<LoadoutRng>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let now = time.elapsed_secs();
	if players.is_empty() && respawn.player_at.is_some_and(|at| now >= at) {
		respawn.player_at = None;
		match session.mode {
			RangeMode::FreeForAll => {
				session::spawn_generated_player(
					&mut commands,
					&spawn,
					&mut rng.0,
					&mut meshes,
					&mut materials,
				);
			}
			RangeMode::Duel => {
				spawn_player_at(&mut commands, &spawn, &mut meshes, &mut materials);
			}
		}
	}
	let mut due = 0usize;
	respawn.npc_at.retain(|at| {
		if now >= *at {
			due += 1;
			false
		} else {
			true
		}
	});
	let live = npcs.iter().count();
	let want = session.npc_count as usize;
	let n = due.min(want.saturating_sub(live));
	for index in 0..n {
		let slot = (live + index) as u16;
		match session.mode {
			RangeMode::FreeForAll => {
				session::spawn_generated_npc(
					&mut commands,
					&spawn,
					slot,
					session.npc_count,
					&mut rng.0,
					&mut meshes,
					&mut materials,
				);
			}
			RangeMode::Duel => {
				spawn_npc_at(&mut commands, &spawn, &mut meshes, &mut materials);
			}
		}
	}
}

fn spawn_reticle_system(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_reticle(&mut commands, &mut meshes, &mut materials);
}

fn gate_pad(focus: Res<TextEntryFocus>, mut enabled: ResMut<PadGameplayEnabled>) {
	enabled.0 = !focus.0;
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.spawn((
		DirectionalLight { illuminance: 2500.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 200.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 3.0, 0.0)),
	));
}
