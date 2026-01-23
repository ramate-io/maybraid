pub mod shaders;
pub mod water;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use std::f32::consts::PI;

mod camera;
mod terrain;
mod ui;

use engine::{
	manage_chunks, shaders::outline::EdgeMaterial, ChunkConfig, ChunkResolutionConfig,
	LoadedChunks, SdfResource,
};

pub use camera::CameraController;
pub use terrain::TerrainConfig;

pub use sdf;

pub struct TerrainPlugin {
	pub seed: u32,
}

impl Plugin for TerrainPlugin {
	fn build(&self, app: &mut App) {
		// Register EdgeMaterial plugin
		app.add_plugins(FrameTimeDiagnosticsPlugin::default());
		app.add_plugins(LogDiagnosticsPlugin::default());
		app.add_plugins(water::WaterPlaygroundPlugin);

		app.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			// forest
			.add_systems(Startup, (camera::setup_camera, setup_lighting, ui::setup_debug_ui))
			.add_systems(Update, (camera::camera_controller, ui::update_coordinate_display));
	}
}

fn setup_lighting(mut commands: Commands) {
	// Main directional light (sun) - primary light source
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadows_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));

	// Fill light from opposite direction - reduces harsh shadows
	commands.spawn((
		DirectionalLight {
			illuminance: 500.0,     // Increased fill light
			shadows_enabled: false, // No shadows for fill light
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));

	// Additional fill lights from sides for omnidirectional illumination
	// Left side
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadows_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, PI / 2.0, 0.0)),
	));

	// Right side
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadows_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, -PI / 2.0, 0.0)),
	));

	// Top-down fill light
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadows_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 2.0, 0.0, 0.0)),
	));
}
