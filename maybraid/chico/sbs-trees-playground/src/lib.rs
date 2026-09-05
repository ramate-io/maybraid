//! Interactive viewer for streamed Chico forests.

pub mod camera;
pub mod checkerboard_material;
pub mod commands;
pub mod diagnostics;
mod ground;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use diagnostics::{LogFps, PlaygroundDiag};
pub use game_commands::command::PendingStartupCommand;

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use chico_forests::{
	drive_forest_stream, FlatForestPresenter, ForestPlugin, ForestStream, ForestStreamSpec,
};
use chico_vegetation_components::{FoliageLodProbe, StickLodProbe};
use commands::RequestMeshStats;
use diagnostics::toggle_fps_logging;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::GameCommandStatusText;
use ground::{setup_ground, GroundPlane};
use lod::LodSceneHost;

pub struct SbsTreesPlaygroundPlugin;

impl Plugin for SbsTreesPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(ForestStream(Some(ForestStreamSpec::default())))
			.add_plugins(ForestPlugin::<FlatForestPresenter>::default())
			.add_plugins(LogFps { enabled: true })
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.add_plugins(
				bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default(),
			)
			.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::camera_controller,
					follow_ground.before(drive_forest_stream),
					toggle_fps_logging.after(capture_command_line_input::<PlaygroundCommand>),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			)
			.add_systems(PostUpdate, apply_mesh_stats.after(VisibilitySystems::CheckVisibility));
	}
}

fn follow_ground(
	camera: Query<&Transform, With<Camera3d>>,
	mut ground: Query<&mut Transform, (With<GroundPlane>, Without<Camera3d>)>,
) {
	let Ok(cam) = camera.single() else {
		return;
	};
	if let Ok(mut ground_tf) = ground.single_mut() {
		ground_tf.translation.x = cam.translation.x;
		ground_tf.translation.z = cam.translation.z;
	}
}

/// Count total vs view-visible mesh triangles (`ViewVisibility`) and LOD probe hosts.
fn apply_mesh_stats(
	mut commands: Commands,
	mut status: ResMut<GameCommandStatusText>,
	mesh_assets: Res<Assets<Mesh>>,
	requests: Query<Entity, With<RequestMeshStats>>,
	mesh_entities: Query<(&Mesh3d, &ViewVisibility)>,
	foliage_probes: Query<(), With<FoliageLodProbe>>,
	stick_probes: Query<(), With<StickLodProbe>>,
	lod_hosts: Query<(), With<LodSceneHost>>,
) {
	for entity in &requests {
		let mut total_entities = 0usize;
		let mut visible_entities = 0usize;
		let mut missing = 0usize;
		let mut total_tris = 0usize;
		let mut visible_tris = 0usize;
		let mut unique_handles = std::collections::HashSet::new();
		let mut visible_unique_handles = std::collections::HashSet::new();

		for (mesh3d, view_visibility) in &mesh_entities {
			total_entities += 1;
			unique_handles.insert(mesh3d.0.id());
			let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
				missing += 1;
				continue;
			};
			let verts = mesh.count_vertices();
			let index_count = mesh.indices().map(|i| i.len()).unwrap_or(verts);
			let tris = index_count / 3;
			total_tris += tris;
			if view_visibility.get() {
				visible_entities += 1;
				visible_unique_handles.insert(mesh3d.0.id());
				visible_tris += tris;
			}
		}

		let foliage_probes = foliage_probes.iter().count();
		let stick_probes = stick_probes.iter().count();
		let lod_hosts = lod_hosts.iter().count();
		let probes_total = foliage_probes + stick_probes;

		status.0 = format!(
			"stats mesh:\n  total_tris={total_tris}\n  visible_tris={visible_tris}\n  entities={total_entities} visible_entities={visible_entities} unique_handles={} visible_unique={} missing={missing}\n  probes: foliage={foliage_probes} stick={stick_probes} total={probes_total}\n  lod_hosts={lod_hosts}",
			unique_handles.len(),
			visible_unique_handles.len(),
		);
		info!("{}", status.0);
		commands.entity(entity).despawn();
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight { brightness: 450.0, ..default() });
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 3500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}
