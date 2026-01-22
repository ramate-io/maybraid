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
		app.add_plugins(bevy::pbr::MaterialPlugin::<EdgeMaterial>::default());

		// Set up geographic features
		let terrain_chunk_config = ChunkConfig::<terrain::TerrainSdf>::default();
		let terrain_resolution_config = ChunkResolutionConfig::<terrain::TerrainSdf>::default();
		let terrain_config = TerrainConfig::new(self.seed);
		let terrain_sdf = terrain::TerrainSdf { sdf: terrain::create_terrain_sdf(&terrain_config) };
		let terrain_sdf_resource = SdfResource::new(terrain_sdf);

		app.insert_resource(terrain_config)
			.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(LoadedChunks::default())
			// terrain
			.insert_resource(terrain_chunk_config)
			.insert_resource(terrain_resolution_config)
			.insert_resource(terrain_sdf_resource)
			// forest
			.add_systems(Startup, (camera::setup_camera, setup_lighting, ui::setup_debug_ui))
			.add_systems(
				Update,
				(
					camera::camera_controller,
					manage_chunks::<terrain::TerrainSdf>,
					ui::update_coordinate_display,
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	// Ambient light - significantly increased to simulate global illumination
	// This provides base lighting for all surfaces, including back faces
	commands.insert_resource(AmbientLight {
		color: Color::WHITE,
		brightness: 2.0, // Much higher for better back-face illumination (simulates bounced light)
		affects_lightmapped_meshes: true,
	});

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
