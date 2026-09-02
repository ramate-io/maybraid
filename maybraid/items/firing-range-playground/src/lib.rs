//! Firing range: pad + Les Halles stack → player + firearm-user plugins.

mod buildings_lod;
pub mod commands;
mod les_halles;
mod range;
mod ui;
mod vantage;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use buildings_lod::FiringRangeBuildingsLodPlugin;
use crozon_characters::{
	species::braidman::BraidmanConfig, CharacterHostsPlugin, CharacterRecipe, CharacterRoot,
};
use firearm_user::{spawn_held_firearm, spawn_reticle, FirearmUserPlugin};
use firearms::{FirearmHostsPlugin, FirearmWeaponsPlugin};
use game_commands::command::{GameCommandPlugin, TextEntryFocus};
use les_halles::LesHallesSpawn;
use lod::LodRefreshSystems;
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_input::{PadGameplayEnabled, VirtualPadSystems};
use movement_intelligence::{
	CandidateBudget, MovementIntelligence, MovementIntelligenceLimits, MovementIntelligencePlugin,
	MovementIntelligenceSystems, ReplanMovement,
};
use movement_intelligence_richmond::RichmondAvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use player::{
	needs_npc_visual, needs_player_visual, spawn_npc_visual, spawn_npc_with_hidden_capsule,
	spawn_player_visual, spawn_player_with_hidden_capsule, Npc, Player, PlayerLook, PlayerPlugin,
	PlayerVisual,
};
use player_camera::{spawn_follow_camera, PlayerCameraPlugin};
use richmond_building_components::{
	apply_parent_confines, FurnitureWireframePlugin, LabelWireframePlugin,
};
use richmond_building_physics::BuildingWalkColliderPlugin;
use std::f32::consts::FRAC_PI_2;

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
		.add_plugins(MovementRealizationPlugin)
		.init_resource::<vantage::NpcVantageRefresh>()
		.init_resource::<LesHallesSpawn>()
		.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
		.add_systems(
			Startup,
			(spawn_follow_camera_system, setup_lighting, range::setup_range, spawn_reticle_system)
				.chain(),
		)
		.add_systems(
			PostStartup,
			(
				les_halles::setup_les_halles,
				spawn_player_system,
				spawn_npc_system,
				spawn_held_system,
			)
				.chain(),
		)
		.add_systems(PreUpdate, gate_pad.before(VirtualPadSystems::Produce))
		.add_systems(
			Update,
			(
				spawn_player_character,
				spawn_npc_character,
				vantage::refresh_npc_vantage.before(MovementIntelligenceSystems::Replan),
				les_halles::draw_circulation_gizmos,
				apply_parent_confines.after(LodRefreshSystems::Cull),
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
			),
		);
	}
}

fn spawn_follow_camera_system(mut commands: Commands) {
	spawn_follow_camera(&mut commands);
}

fn spawn_player_system(
	mut commands: Commands,
	spawn: Res<LesHallesSpawn>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let player = spawn_player_with_hidden_capsule(&mut commands, &mut meshes, &mut materials);
	commands.entity(player).insert((
		Transform::from_translation(spawn.player),
		PlayerLook { yaw: spawn.look_yaw, ..default() },
	));
}

fn spawn_npc_system(
	mut commands: Commands,
	spawn: Res<LesHallesSpawn>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let npc = spawn_npc_with_hidden_capsule(
		&mut commands,
		spawn.npc,
		PlayerLook { yaw: spawn.look_yaw, ..default() },
		&mut meshes,
		&mut materials,
	);
	let mut brain = MovementIntelligence::new(vantage::vantage_on_player(spawn.player));
	brain.ability.candidate_budget.horizon = 80.0;
	commands.entity(npc).insert((brain, ReplanMovement));
}

fn spawn_held_system(mut commands: Commands, bodies: Query<Entity, Or<(With<Player>, With<Npc>)>>) {
	for body in &bodies {
		spawn_held_firearm(&mut commands, body);
	}
}

fn spawn_reticle_system(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_reticle(&mut commands, &mut meshes, &mut materials);
}

fn spawn_npc_character(
	mut commands: Commands,
	npcs: Query<Entity, With<Npc>>,
	visuals: Query<&ChildOf, With<CharacterRoot>>,
) {
	let Some(npc) = needs_npc_visual(npcs, visuals) else {
		return;
	};
	let clothed = CharacterRecipe::clothed(&BraidmanConfig::default_preview());
	spawn_npc_visual(&mut commands, npc, clothed, Quat::from_rotation_y(FRAC_PI_2));
}

fn spawn_player_character(
	mut commands: Commands,
	players: Query<Entity, With<Player>>,
	visuals: Query<&ChildOf, With<PlayerVisual>>,
) {
	let Some(player) = needs_player_visual(players, visuals) else {
		return;
	};
	let clothed = CharacterRecipe::clothed(&BraidmanConfig::default_preview());
	spawn_player_visual(&mut commands, player, clothed, Quat::from_rotation_y(FRAC_PI_2));
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
