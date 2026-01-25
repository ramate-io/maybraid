pub mod shaders;
pub mod terrain;
pub mod water;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::globals::GlobalsPlugin;
use engine::shaders::outline::EdgeMaterial;
use std::f32::consts::PI;

mod camera;
mod ui;

pub use camera::CameraController;

pub use sdf;

pub struct NatureScapesPlugin;

impl Plugin for NatureScapesPlugin {
	fn build(&self, app: &mut App) {
		// Register EdgeMaterial plugin
		app.add_plugins(bevy::pbr::MaterialPlugin::<EdgeMaterial>::default());
		app.add_plugins(FrameTimeDiagnosticsPlugin::default());
		app.add_plugins(LogDiagnosticsPlugin::default());
		app.add_plugins(water::WaterPlaygroundPlugin);
		app.add_plugins(terrain::TerrainPlaygroundPlugin { material: EdgeMaterial::default() });

		app.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			// forest
			.add_systems(Startup, (camera::setup_camera, setup_lighting, ui::setup_debug_ui))
			.add_systems(Update, (camera::camera_controller, ui::update_coordinate_display));
	}
}

fn setup_lighting(mut commands: Commands) {
	// Main directional light (sun) - primary light source
	commands.spawn((
		PointLight {
			radius: 20.0,
			intensity: 10000000000.0,
			range: 100000.0,
			shadows_enabled: true,
			..default()
		},
		// high in the sky
		Transform::from_xyz(100.0, 100.0, 100.0),
	));
}
