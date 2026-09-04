//! Assembled world model: Durham terrain, streamed forest, urbanization, sky dome.
//!
//! Character mode is the default. Forest grove fill is 1 km present / 3 km
//! selection generate. Canopy bump-outs occupy the 1–5 km present keep and
//! clone Durham fine-cell mesh handles. Vegetation LOD bullseye / lattice
//! cover the grove fill ring. Urbanization hopscotch streams at the same
//! 1 km / 3 km rings without re-registering Durham (vegetation owns terrain).

mod camera;
pub mod commands;
mod control;
mod material_lib;
mod ui;

pub use camera::CameraPov;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use control::WorldGameplayEnabled;
pub use game_commands::command::PendingStartupCommand;
pub use material_lib::{WorldMaterialLib, WorldMaterialRefPlugin};

use avian3d::prelude::{CoefficientCombine, Friction};
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	CharacterCameraFollowEnabled, CharacterLocomotion, CharacterSpecies, PadMovementEnabled,
	PlayerControlSystems, PlaygroundConfig as VegetationPlaygroundConfig, PlaygroundDiag,
	PlaygroundMode, PlaygroundTimingPlugin, RequestSetCharacter, VegetationOnTerrainPlugin,
};
use durham_terrain_models::TerrainFrictionConfig;
use game_commands::command::{GameCommandPlugin, TextEntryFocus};
use game_commands::ui::GameCommandDrawerConfig;
use lod::{Bullseye, OpenLattice};
use maybraid_character_controller::{CharacterControlSystems, CharacterControllerPlugin};
use maybraid_input::{VirtualPadConfig, VirtualPadPlugin};
use maybraid_sky::SkyDomePlugin;
use richmond_developments_on_terrain_playground::{
	DevelopmentsOnTerrainPlugin, PlaygroundConfig as DevelopmentsPlaygroundConfig,
};

/// Steepest walkable slope. 80°+ walls must not count as floor.
const WORLD_MAX_SLOPE_ANGLE: f32 = 50.0_f32.to_radians();
/// Static grip sits above `tan(50°)` ≈ 1.19 so walkable slopes do not ice-skate.
const WORLD_TERRAIN_FRICTION: Friction = Friction {
	dynamic_coefficient: 1.35,
	static_coefficient: 1.6,
	combine_rule: CoefficientCombine::Max,
};

/// ±1 km so produce covers the 1 km grove present ring.
const WORLD_BULLSEYE_OUTER_M: f32 = 2_000.0;
/// Cull annulus starts beyond the present ring.
const WORLD_LATTICE_EXCLUDE_M: f32 = 2_000.0;
const WORLD_LATTICE_OUTER_M: f32 = 8_000.0;

/// Assembled world: Durham terrain, streamed forest, urbanization, sky dome, character.
///
/// Playground chrome (command drawer, FPS HUD, pad dump) is on by default.
/// The game executable uses [`WorldPlugin::game`].
pub struct WorldPlugin {
	/// `/` console, FPS HUD, and virtual-pad dump.
	pub debug_chrome: bool,
}

impl Default for WorldPlugin {
	fn default() -> Self {
		Self { debug_chrome: true }
	}
}

impl WorldPlugin {
	/// World systems without playground overlays.
	pub fn game() -> Self {
		Self { debug_chrome: false }
	}
}

impl Plugin for WorldPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(PlaygroundMode::Character)
			.insert_resource(PlaygroundDiag { fps: self.debug_chrome })
			.insert_resource(CameraPov::default())
			.insert_resource(CharacterLocomotion { max_slope_angle: WORLD_MAX_SLOPE_ANGLE })
			.insert_resource(TerrainFrictionConfig(WORLD_TERRAIN_FRICTION))
			.add_plugins(WorldMaterialRefPlugin)
			.add_plugins(VirtualPadPlugin::new(VirtualPadConfig {
				debug_overlay: self.debug_chrome,
				..default()
			}))
			.add_plugins(CharacterControllerPlugin)
			.add_plugins(VegetationOnTerrainPlugin {
				config: VegetationPlaygroundConfig::world_defaults(),
				commands: false,
				register_forest_lod: false,
				register_bump_out_lod: false,
			})
			// Urbanization stream only — vegetation already owns Durham / TerrainEntryStore.
			.add_plugins(DevelopmentsOnTerrainPlugin {
				config: DevelopmentsPlaygroundConfig::world_defaults(),
				commands: false,
				own_terrain: false,
				register_development_forest_lod: true,
			})
			.insert_resource(PadMovementEnabled(false))
			.insert_resource(CharacterCameraFollowEnabled(false))
			.init_resource::<WorldGameplayEnabled>()
			.insert_resource(Bullseye { inner: 50.0, outer: WORLD_BULLSEYE_OUTER_M })
			.insert_resource(OpenLattice {
				exclude_extent: WORLD_LATTICE_EXCLUDE_M,
				outer_extent: WORLD_LATTICE_OUTER_M,
				tile_size: 500.0,
			})
			.add_plugins(SkyDomePlugin::default());
		if self.debug_chrome {
			app.add_plugins(PlaygroundTimingPlugin).add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						toggle_gamepad_buttons: Vec::new(),
					}),
			);
		} else {
			app.init_resource::<TextEntryFocus>();
		}
		app.add_systems(PostStartup, spawn_default_braidman).add_systems(
			Update,
			control::apply_intents_to_movement
				.after(CharacterControlSystems)
				.before(PlayerControlSystems),
		);
		if self.debug_chrome {
			app.add_systems(
				Update,
				(
					control::echo_character_intents
						.after(CharacterControlSystems)
						.before(game_commands::ui::update_debug_ui),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
		}
		app.add_systems(
			PostUpdate,
			camera::turn_body_with_look.before(TransformSystems::Propagate),
		)
		.add_systems(
			PostUpdate,
			(camera::sync_first_person_head_visibility, camera::follow_world_camera)
				.chain()
				.after(TransformSystems::Propagate),
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
