//! Firing range: pad → character controller, held bullpup, trigger fire.

mod camera;
mod character;
pub mod commands;
mod control;
mod hold;
mod player;
mod range;
mod reticle;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use crozon_characters::{CharacterHostsPlugin, CharacterMotionSystems};
use firearms::{FirearmHostsPlugin, FirearmWeaponsPlugin};
use game_commands::command::GameCommandPlugin;
use maybraid_character_controller::{CharacterControlSystems, CharacterControllerPlugin};
use maybraid_input::VirtualPadSystems;

use player::PlayerControlSystems;

pub struct FiringRangePlugin;

impl Plugin for FiringRangePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(FirearmHostsPlugin)
			.add_plugins(FirearmWeaponsPlugin)
			.add_plugins(CharacterHostsPlugin)
			.add_plugins(CharacterControllerPlugin)
			.add_plugins(player::PlayerPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.add_systems(
				Startup,
				(
					camera::setup_camera,
					setup_lighting,
					range::setup_range,
					player::spawn_player,
					character::spawn_held_firearm,
					reticle::spawn_reticle,
				)
					.chain(),
			)
			.add_systems(PreUpdate, control::gate_pad.before(VirtualPadSystems::Produce))
			.add_systems(
				Update,
				(
					camera::release_modifiers_on_focus_change,
					character::spawn_player_character,
					character::stamp_holding_arms,
					control::apply_intents
						.after(CharacterControlSystems)
						.before(PlayerControlSystems),
					control::face_player.after(PlayerControlSystems),
					character::pose_held_firearm.after(control::face_player),
					character::drive_player_locomotion
						.after(PlayerControlSystems)
						.before(CharacterMotionSystems::Anim),
					hold::sync_hands_to_firearm
						.after(CharacterMotionSystems::Anim)
						.after(character::pose_held_firearm),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
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
