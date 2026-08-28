//! Interactive viewer for Chico stalk-and-ball-stick trees.

pub mod camera;
pub mod checkerboard_material;
mod chico_material_lib;
pub mod commands;
pub mod diagnostics;
pub mod forest_stream;
mod ground;
mod monster_grass_plain;
mod render;
mod render_materials;
pub mod stick_physics;
mod ui;
mod vast;
pub mod vegetation_lod;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use diagnostics::{PlaygroundDiag, PlaygroundTimingPlugin};
pub use game_commands::command::PendingStartupCommand;
pub use monster_grass_plain::PLAIN_GROVE_RADIUS;
pub use render::{RenderConfig, RenderSubject};
pub use vast::{VAST_GROVE_RADIUS, VAST_ORCHARD_RADIUS};
pub use vegetation_lod::VegetationLodRefreshPlugin;

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use chico_ball_components::frond::FrondRenderItemPlugin;
use chico_ball_components::tuft::render_item_plugin::TuftRenderItemPlugin;
use chico_material_lib::ChicoMaterialRefPlugin;
use chico_sbs_trees::ensure_chico_tree_render_plugins;
use chico_sdf::{CrookCylinder, NoisyBall, NoisyCylinder};
use chico_vegetation_components::{FoliageLodProbe, StickLodProbe, VegetationProceduralPlugin};
use chico_vegetation_shaders::{
	ChicoLeafMaterial, ChicoStickMaterial, ChicoVegetationShadersPlugin,
};
use commands::show::{sync_show, ShowConfig};
use commands::RequestMeshStats;
use diagnostics::toggle_fps_logging;
use forest_stream::{register_forest_lod, stream_forest, ForestRegionPresenter};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::GameCommandStatusText;
use ground::setup_ground;
use lod::{LodGenerateSystems, LodPresentSystems, LodSceneHost};
use render::sync_render;
use render_item::mesh::handle::EnforceCachingPlugin;
use render_materials::{setup_render_materials, sync_render_material_handles};
use scene_ref::SceneRefPlugin;

/// Shaders, materials, tree render items, and vegetation LOD refresh.
///
/// Shared by the SBS trees playground and vegetation-on-terrain.
pub fn register_vegetation_view(app: &mut App) {
	if !app.is_plugin_added::<SceneRefPlugin>() {
		app.add_plugins(SceneRefPlugin);
	}
	if !app.is_plugin_added::<VegetationProceduralPlugin>() {
		app.add_plugins(VegetationProceduralPlugin);
	}
	if !app.is_plugin_added::<VegetationLodRefreshPlugin>() {
		app.add_plugins(VegetationLodRefreshPlugin);
	}
	ensure_chico_tree_render_plugins(app);
	if !app.is_plugin_added::<TuftRenderItemPlugin>() {
		app.add_plugins(TuftRenderItemPlugin::default());
	}
	if !app.is_plugin_added::<FrondRenderItemPlugin>() {
		app.add_plugins(FrondRenderItemPlugin::default());
	}
	app.add_plugins(ChicoVegetationShadersPlugin);
	if !app.is_plugin_added::<ChicoMaterialRefPlugin>() {
		app.add_plugins(ChicoMaterialRefPlugin);
	}
	ensure_enforce_caching_plugin::<NoisyCylinder, ChicoStickMaterial>(app);
	ensure_enforce_caching_plugin::<CrookCylinder, ChicoStickMaterial>(app);
	ensure_enforce_caching_plugin::<NoisyBall, ChicoLeafMaterial>(app);
	ensure_enforce_caching_plugin::<NoisyBall, ChicoStickMaterial>(app);
	if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
		app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
	}
}

pub struct SbsTreesPlaygroundPlugin;

impl Plugin for SbsTreesPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<RenderConfig>();
		app.init_resource::<ShowConfig>();
		register_vegetation_view(app);
		register_forest_lod::<ForestRegionPresenter>(app);
		if !app.is_plugin_added::<PlaygroundTimingPlugin>() {
			app.add_plugins(PlaygroundTimingPlugin);
		}
		app.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.add_plugins(
				bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default(),
			)
			.add_systems(
				Startup,
				(camera::setup_camera, setup_lighting, setup_ground, setup_render_materials),
			)
			.add_systems(
				Update,
				(
					camera::camera_controller,
					sync_render_material_handles
						.after(capture_command_line_input::<PlaygroundCommand>)
						.before(sync_render),
					sync_render
						.after(capture_command_line_input::<PlaygroundCommand>)
						.after(sync_render_material_handles),
					sync_show.after(capture_command_line_input::<PlaygroundCommand>),
					stream_forest
						.after(sync_show)
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce),
					toggle_fps_logging.after(capture_command_line_input::<PlaygroundCommand>),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			)
			.add_systems(PostUpdate, apply_mesh_stats.after(VisibilitySystems::CheckVisibility));
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

fn ensure_enforce_caching_plugin<T, M>(app: &mut App)
where
	T: render_item::mesh::MeshBuilder
		+ render_item::mesh::IdentifiedMesh
		+ Clone
		+ Send
		+ Sync
		+ 'static,
	M: bevy::prelude::Material + Send + Sync + 'static,
{
	if !app.is_plugin_added::<EnforceCachingPlugin<T, M>>() {
		app.add_plugins(EnforceCachingPlugin::<T, M>::default());
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
