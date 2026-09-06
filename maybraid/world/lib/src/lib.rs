//! Assembled world model: Durham terrain, streamed forest, urbanization, sky dome.
//!
//! Character mode is the default. Forest grove fill is 1 km present / 3 km
//! selection generate. Canopy bump-outs occupy the 1–5 km present keep and
//! clone Durham fine-cell mesh handles. Vegetation LOD bullseye / lattice
//! cover the grove fill ring. Urbanization hopscotch streams at the same
//! 1 km / 3 km rings without re-registering Durham (vegetation owns terrain).

mod camera;
pub mod commands;
mod contact;
mod control;
mod intelligence;
mod material_lib;
mod mobs;
mod pitch;
mod poi;
mod ui;
mod weapon;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use control::WorldGameplayEnabled;
pub use game_commands::command::PendingStartupCommand;
pub use intelligence::WorldIntelligencePlugin;
pub use material_lib::{WorldMaterialLib, WorldMaterialRefPlugin};
pub use mobs::WorldMobsPlugin;
pub use player_camera::CameraPov;
pub use poi::{WorldPoiDiscoveryBudget, WorldPoiPlugin, WorldPoiSystems};

use avian3d::prelude::{CoefficientCombine, Friction, PhysicsPlugins, PhysicsSchedulePlugin};
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	CharacterCameraFollowEnabled, CharacterLocomotion, CharacterSpecies, PadMovementEnabled,
	PlayerControlSystems, PlaygroundConfig as VegetationPlaygroundConfig, PlaygroundDiag,
	PlaygroundMode, PlaygroundTimingPlugin, RequestSetCharacter, VegetationOnTerrainPlugin,
};
use crozon_characters::CharacterMotionSystems;
use durham_terrain_models::TerrainFrictionConfig;
use game_commands::command::{GameCommandPlugin, TextEntryFocus};
use game_commands::ui::GameCommandDrawerConfig;
use lod::{Bullseye, OpenLattice};
use maybraid_character_controller::{CharacterControlSystems, CharacterControllerPlugin};
use maybraid_input::{VirtualPadConfig, VirtualPadPlugin};
use maybraid_sky::SkyDomePlugin;
use player::PlayerPresentationPlugin;
use player_camera::{PlayerCameraPlugin, PlayerCameraSystems};
use richmond_developments_on_terrain_playground::{
	DevelopmentsOnTerrainPlugin, PlaygroundConfig as DevelopmentsPlaygroundConfig,
};

/// Steepest slope the controlled character can drive uphill.
const WORLD_MAX_SLOPE_ANGLE: f32 = 70.0_f32.to_radians();
/// Static grip for mobs and props; controlled-player contacts disable friction.
const WORLD_TERRAIN_FRICTION: Friction = Friction {
	dynamic_coefficient: 2.55,
	static_coefficient: 2.95,
	combine_rule: CoefficientCombine::Max,
};

/// ±1 km so produce covers the 1 km grove present ring.
const WORLD_BULLSEYE_OUTER_M: f32 = 2_000.0;
/// Cull annulus starts beyond the present ring.
const WORLD_LATTICE_EXCLUDE_M: f32 = 2_000.0;
const WORLD_LATTICE_OUTER_M: f32 = 8_000.0;

/// Assembled world: Durham terrain, streamed forest, urbanization, sky dome, character.
///
/// Playground chrome (command drawer and FPS HUD) is on by default.
/// The game executable uses [`WorldPlugin::game`].
pub struct WorldPlugin {
	/// `/` console and FPS HUD.
	pub debug_chrome: bool,
	/// Upper-left virtual-pad / command-intent dump.
	pub input_debug_enabled: bool,
}

impl Default for WorldPlugin {
	fn default() -> Self {
		Self { debug_chrome: true, input_debug_enabled: false }
	}
}

impl WorldPlugin {
	/// World systems without playground overlays.
	pub fn game() -> Self {
		Self { debug_chrome: false, input_debug_enabled: false }
	}
}

impl Plugin for WorldPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(
				PhysicsPlugins::default().with_collision_hooks::<contact::WorldCollisionHooks>(),
			);
		}
		app.insert_resource(PlaygroundMode::Character)
			.insert_resource(PlaygroundDiag { fps: self.debug_chrome })
			.insert_resource(CharacterLocomotion { max_slope_angle: WORLD_MAX_SLOPE_ANGLE })
			.insert_resource(player::CharacterLocomotion { max_slope_angle: WORLD_MAX_SLOPE_ANGLE })
			.insert_resource(TerrainFrictionConfig(WORLD_TERRAIN_FRICTION))
			.add_plugins(WorldMaterialRefPlugin)
			.add_plugins(VirtualPadPlugin::new(VirtualPadConfig {
				debug_overlay: self.input_debug_enabled,
				..default()
			}))
			.add_plugins(CharacterControllerPlugin)
			.add_plugins(PlayerPresentationPlugin)
			.add_plugins(PlayerCameraPlugin)
			.add_plugins(VegetationOnTerrainPlugin {
				config: VegetationPlaygroundConfig::world_defaults(),
				commands: false,
				register_forest_lod: false,
				register_bump_out_lod: false,
				register_camera: false,
				register_terrain_pitch: false,
			})
			// Urbanization stream only — vegetation already owns Durham / TerrainEntryStore.
			.add_plugins(DevelopmentsOnTerrainPlugin {
				config: DevelopmentsPlaygroundConfig::world_defaults(),
				commands: false,
				own_terrain: false,
				register_development_forest_lod: true,
			})
			.add_plugins(WorldMobsPlugin)
			.add_plugins(WorldIntelligencePlugin)
			.add_plugins(WorldPoiPlugin)
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
		camera::configure(app);
		weapon::configure(app);
		app.configure_sets(
			Update,
			PlayerCameraSystems::Body
				.after(CharacterMotionSystems::Anim)
				.before(CharacterMotionSystems::Elevation),
		);
		app.add_systems(
			Update,
			(
				pitch::sync_suspend_terrain_pitch.after(PlayerControlSystems),
				pitch::apply_avian_terrain_pitch
					.in_set(CharacterMotionSystems::Elevation)
					.after(pitch::sync_suspend_terrain_pitch),
			),
		);
		if self.debug_chrome {
			app.add_systems(Startup, ui::spawn_mob_debug_hud).add_systems(
				Update,
				(
					control::echo_character_intents
						.after(CharacterControlSystems)
						.before(game_commands::ui::update_debug_ui),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
					ui::sync_mob_debug_pins,
					ui::draw_mob_debug_gizmos,
				),
			);
		}
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
		assert!(degrees > 55.0);
	}

	#[test]
	fn world_terrain_static_friction_holds_max_walkable_slope() {
		assert!(WORLD_TERRAIN_FRICTION.static_coefficient > WORLD_MAX_SLOPE_ANGLE.tan());
	}

	#[test]
	fn world_input_debug_overlay_is_opt_in() {
		assert!(!WorldPlugin::default().input_debug_enabled);
		assert!(!WorldPlugin::game().input_debug_enabled);
	}
}
