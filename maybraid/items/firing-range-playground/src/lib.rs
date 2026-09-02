//! Firing range: pad → player + firearm-user plugins, range geometry.

pub mod commands;
mod range;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use crozon_characters::{species::braidman::BraidmanConfig, CharacterHostsPlugin, CharacterRecipe};
use firearm_user::{spawn_held_firearm, spawn_reticle, FirearmUserPlugin};
use firearms::{FirearmHostsPlugin, FirearmWeaponsPlugin};
use game_commands::command::{GameCommandPlugin, TextEntryFocus};
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_input::{PadGameplayEnabled, VirtualPadSystems};
use maybraid_player::{
	needs_player_visual, spawn_player_visual, spawn_player_with_hidden_capsule, Player,
	PlayerPlugin, PlayerVisual,
};
use maybraid_player_camera::{spawn_follow_camera, PlayerCameraPlugin};
use std::f32::consts::FRAC_PI_2;

pub struct FiringRangePlugin;

impl Plugin for FiringRangePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(FirearmHostsPlugin)
			.add_plugins(FirearmWeaponsPlugin)
			.add_plugins(CharacterHostsPlugin)
			.add_plugins(CharacterControllerPlugin)
			.add_plugins(PlayerPlugin)
			.add_plugins(PlayerCameraPlugin)
			.add_plugins(FirearmUserPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.add_systems(
				Startup,
				(
					spawn_follow_camera_system,
					setup_lighting,
					range::setup_range,
					spawn_player_system,
					spawn_held_system,
					spawn_reticle_system,
				)
					.chain(),
			)
			.add_systems(PreUpdate, gate_pad.before(VirtualPadSystems::Produce))
			.add_systems(
				Update,
				(
					spawn_player_character,
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
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_player_with_hidden_capsule(&mut commands, &mut meshes, &mut materials);
}

fn spawn_held_system(mut commands: Commands, players: Query<Entity, With<Player>>) {
	let Ok(player) = players.single() else {
		return;
	};
	spawn_held_firearm(&mut commands, player);
}

fn spawn_reticle_system(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_reticle(&mut commands, &mut meshes, &mut materials);
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
