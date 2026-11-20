//! BotW-like GPU tree-canopy playground: orbit, grove edge, slopes, LOD, wind.

mod camera;
mod capture;
mod commands;
mod ground;
mod kit;
mod scene;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use bevy::window::{PresentMode, Window, WindowResolution};
use camera_controls::look::CameraLookPlugin;
use capture::CaptureSession;
use game_commands::command::GameCommandPlugin;
use terrain_shaders::TerrainShaderPlugin;
use vegetation_components::CanopyShadowMode;
use vegetation_grass_field::GrassFieldPlugin;
use vegetation_tree_field::TreeFieldPlugin;

pub fn run() {
	let capturing = CaptureSession::from_env();
	let resolution = if capturing.is_some() {
		let (w, h) = CaptureSession::window_resolution();
		WindowResolution::new(w, h).with_scale_factor_override(1.0)
	} else {
		WindowResolution::new(1280, 720)
	};
	let mut app = App::new();
	app.add_plugins(DefaultPlugins.set(WindowPlugin {
		primary_window: Some(Window {
			title: "vegetation-tree-field-playground".into(),
			resolution,
			present_mode: PresentMode::AutoNoVsync,
			..default()
		}),
		..default()
	}))
	.insert_resource(ClearColor(Color::srgb(0.62, 0.78, 0.92)))
	.insert_resource(CanopyShadowMode::Full)
	.add_plugins(TerrainShaderPlugin)
	.add_plugins(GrassFieldPlugin)
	.add_plugins(TreeFieldPlugin)
	.add_plugins(CameraLookPlugin::default())
	.add_plugins(
		GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()).with_drawer_config(
			game_commands::ui::GameCommandDrawerConfig {
				open_at_start: capturing.is_none(),
				..default()
			},
		),
	)
	.add_systems(
		Startup,
		(setup_lighting, camera::setup_camera, ground::setup_ground, scene::spawn_default_scene),
	)
	.add_systems(
		Update,
		(
			camera::camera_controller.run_if(not(resource_exists::<CaptureSession>)),
			ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
		),
	);
	if let Some(session) = capturing {
		app.insert_resource(session).add_systems(Update, capture::tick_capture);
	}
	app.run();
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight {
		color: Color::srgb(0.72, 0.82, 0.95),
		brightness: 280.0,
		affects_lightmapped_meshes: true,
	});
	commands.spawn((
		DirectionalLight {
			illuminance: 12_000.0,
			color: Color::srgb(1.0, 0.94, 0.82),
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.2, PI / 5.0, 0.0)),
		capture::KeySun,
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 3_200.0,
			color: Color::srgb(0.55, 0.70, 1.0),
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 5.0, -PI / 4.0, 0.0)),
		capture::FillSky,
	));
}
