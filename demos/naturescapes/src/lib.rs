pub mod shaders;
pub mod terrain;
pub mod vegetation;
pub mod water;

// use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin};
use engine::shaders::{leaf_material::LeafMaterial, outline::EdgeMaterial};
use terrain::TerrainSdf;

mod camera;
mod ui;

pub use camera::CameraController;

pub use sdf;

pub struct NatureScapesPlugin;

impl Plugin for NatureScapesPlugin {
	fn build(&self, app: &mut App) {
		// Register material plugins
		app.add_plugins(bevy::pbr::MaterialPlugin::<EdgeMaterial>::default());
		app.add_plugins(DurhamTerrainShaderPlugin);
		app.add_plugins(bevy::pbr::MaterialPlugin::<LeafMaterial>::default());
		// app.add_plugins(FrameTimeDiagnosticsPlugin::default());
		// app.add_plugins(LogDiagnosticsPlugin::default());
		app.add_plugins(water::WaterPlaygroundPlugin);
		app.add_plugins(terrain::TerrainPlaygroundPlugin {
			material: DurhamTerrainShader::default(),
			rock_detail_material: DurhamTerrainShader::default(),
			second_rock_detail_material: DurhamTerrainShader::default(),
			tuft_detail_material: DurhamTerrainShader::default(),
		});
		app.add_plugins(vegetation::VegetationPlaygroundPlugin::<
			EdgeMaterial,
			LeafMaterial,
			TerrainSdf,
		>::new(
			// slightly dark brown color
			EdgeMaterial::default().with_base_color(Vec4::new(0.5, 0.4, 0.2, 1.0)),
			LeafMaterial::default(),
		));

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
			radius: 200.0,
			intensity: 1000000000000000.0,
			range: 1_000_000.0,
			shadows_enabled: true,
			..default()
		},
		// high in the sky
		Transform::from_xyz(100_000.0, 100_000.0, 100_000.0),
	));
}
