use bevy::prelude::*;
use std::f32::consts::PI;

pub mod buildings_playground;
mod camera;
mod checkerboard_material;
mod ground;
pub mod tree;
mod ui;

use buildings::complex::render::ComplexRenderer;
use buildings::meshes::walls::wall::{Wall, WallMesh};
use engine::shaders::{leaf_material::LeafMaterial, outline::EdgeMaterial};
use render_item::{
	mesh::{fetch_meshes, handle::MeshHandle},
	render_items,
};
use vegetation_sdf::{
	grove::Grove,
	tree::{
		meshes::canopy::ball::NoisyBall, meshes::trunk::segment::SimpleTrunkSegment, TreeRenderItem,
	},
};

pub use camera::CameraController;

pub use sdf;

pub struct SkillMapPlugin {
	pub seed: u32,
}

impl Plugin for SkillMapPlugin {
	fn build(&self, app: &mut App) {
		// Register EdgeMaterial plugin
		app.add_plugins(bevy::pbr::MaterialPlugin::<EdgeMaterial>::default());
		app.add_plugins(bevy::pbr::MaterialPlugin::<LeafMaterial>::default());
		// Register CheckerboardMaterial plugin
		app.add_plugins(
			bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default(),
		);

		app.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(ground::CheckerSize::default())
			.add_systems(
				Startup,
				(
					camera::setup_camera,
					setup_lighting,
					ground::setup_ground,
					ui::setup_debug_ui,
					tree::setup_tree_edge_material,
					buildings_playground::setup_buildings_material,
				),
			)
			.add_systems(
				Update,
				(
					camera::camera_controller,
					ground::update_checker_size,
					ui::update_coordinate_display,
					render_items::<TreeRenderItem<EdgeMaterial, LeafMaterial>>,
					render_items::<Grove<EdgeMaterial, LeafMaterial>>,
					fetch_meshes::<MeshHandle<SimpleTrunkSegment>, EdgeMaterial>,
					fetch_meshes::<MeshHandle<NoisyBall>, LeafMaterial>,
					tree::tree_playground::<EdgeMaterial, LeafMaterial>
						.run_if(resource_exists::<tree::TreeMaterial<EdgeMaterial>>)
						.run_if(run_once),
					render_items::<ComplexRenderer<Wall<EdgeMaterial>, Wall<EdgeMaterial>>>,
					fetch_meshes::<MeshHandle<WallMesh>, EdgeMaterial>,
					buildings_playground::building_playground::<EdgeMaterial, EdgeMaterial>
						.run_if(
							resource_exists::<buildings_playground::BuildingMaterial<EdgeMaterial>>,
						)
						.run_if(run_once),
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
