//! Assembled world model: Durham terrain, streamed forest, sky dome.
//!
//! Character mode is the default. Forest present / generate are 2 km / 3 km;
//! vegetation LOD bullseye / lattice are widened to match.

mod camera;
pub mod commands;
mod control;
mod ui;

pub use camera::CameraPov;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use avian3d::prelude::{CoefficientCombine, Friction};
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	CharacterLocomotion, CharacterSpecies, PadMovementEnabled, PlayerControlSystems,
	PlaygroundConfig, PlaygroundDiag, PlaygroundMode, PlaygroundTimingPlugin, RequestSetCharacter,
	VegetationOnTerrainPlugin,
};
use durham_terrain_models::TerrainFrictionConfig;
use game_commands::command::GameCommandPlugin;
use game_commands::ui::GameCommandDrawerConfig;
use lod::{Bullseye, OpenLattice};
use maybraid_character_controller::{CharacterControlSystems, CharacterControllerPlugin};
use maybraid_input::{VirtualPadConfig, VirtualPadPlugin};
use maybraid_sky::SkyDomePlugin;

/// Steepest walkable slope. 80°+ walls must not count as floor.
const WORLD_MAX_SLOPE_ANGLE: f32 = 50.0_f32.to_radians();
/// Static grip sits above `tan(50°)` ≈ 1.19 so walkable slopes do not ice-skate.
const WORLD_TERRAIN_FRICTION: Friction = Friction {
	dynamic_coefficient: 1.35,
	static_coefficient: 1.6,
	combine_rule: CoefficientCombine::Max,
};

/// ±2 km so produce covers the 2 km present ring (stream-radius 2).
const WORLD_BULLSEYE_OUTER_M: f32 = 4_000.0;
/// Cull annulus starts beyond the present ring.
const WORLD_LATTICE_EXCLUDE_M: f32 = 2_000.0;
const WORLD_LATTICE_OUTER_M: f32 = 8_000.0;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(PlaygroundMode::Character)
			.insert_resource(PlaygroundDiag { fps: true })
			.insert_resource(CameraPov::default())
			.insert_resource(CharacterLocomotion { max_slope_angle: WORLD_MAX_SLOPE_ANGLE })
			.insert_resource(TerrainFrictionConfig(WORLD_TERRAIN_FRICTION))
			.add_plugins(VirtualPadPlugin::new(VirtualPadConfig {
				debug_overlay: true,
				..default()
			}))
			.add_plugins(CharacterControllerPlugin)
			.add_plugins(VegetationOnTerrainPlugin {
				config: PlaygroundConfig::world_defaults(),
				commands: false,
			})
			.insert_resource(PadMovementEnabled(false))
			.insert_resource(Bullseye { inner: 50.0, outer: WORLD_BULLSEYE_OUTER_M })
			.insert_resource(OpenLattice {
				exclude_extent: WORLD_LATTICE_EXCLUDE_M,
				outer_extent: WORLD_LATTICE_OUTER_M,
				tile_size: 500.0,
			})
			.add_plugins(SkyDomePlugin::default())
			.add_plugins(PlaygroundTimingPlugin)
			.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						..default()
					}),
			)
			.add_systems(PostStartup, spawn_default_braidman)
			.add_systems(
				Update,
				(
					control::apply_intents_to_movement
						.after(CharacterControlSystems)
						.before(PlayerControlSystems),
					control::echo_character_intents
						.after(CharacterControlSystems)
						.before(game_commands::ui::update_debug_ui),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			)
			.add_systems(
				PostUpdate,
				(camera::follow_world_camera, camera::sync_pov_visibility).chain(),
			);
	}
}

fn spawn_default_braidman(mut commands: Commands) {
	commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn world_slope_is_below_wall_grade() {
		let degrees = WORLD_MAX_SLOPE_ANGLE.to_degrees();
		assert!(degrees < 80.0);
		assert!(degrees > 40.0);
	}

	#[test]
	fn world_terrain_static_friction_holds_max_walkable_slope() {
		assert!(WORLD_TERRAIN_FRICTION.static_coefficient > WORLD_MAX_SLOPE_ANGLE.tan());
	}
}
