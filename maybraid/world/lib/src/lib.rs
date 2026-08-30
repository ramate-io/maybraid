//! Assembled world model: Durham terrain, streamed forest, sky dome.
//!
//! Character mode is the default. Forest grove fill is 1 km present / 3 km
//! selection generate. Canopy bump-outs occupy the 1–3 km annulus. Vegetation
//! LOD bullseye / lattice cover the grove fill ring.

pub mod commands;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	CharacterSpecies, PlaygroundConfig, PlaygroundDiag, PlaygroundMode, PlaygroundTimingPlugin,
	RequestSetCharacter, VegetationOnTerrainPlugin,
};
use game_commands::command::GameCommandPlugin;
use game_commands::ui::GameCommandDrawerConfig;
use lod::{Bullseye, OpenLattice};
use maybraid_sky::SkyDomePlugin;

/// ±1 km so produce covers the 1 km grove present ring.
const WORLD_BULLSEYE_OUTER_M: f32 = 2_000.0;
/// Cull annulus starts beyond the present ring.
const WORLD_LATTICE_EXCLUDE_M: f32 = 2_000.0;
const WORLD_LATTICE_OUTER_M: f32 = 8_000.0;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(PlaygroundMode::Character)
			.insert_resource(PlaygroundDiag { fps: true })
			.add_plugins(VegetationOnTerrainPlugin {
				config: PlaygroundConfig::world_defaults(),
				commands: false,
			})
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
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
			);
	}
}

fn spawn_default_braidman(mut commands: Commands) {
	commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
}
