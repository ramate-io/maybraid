//! Interactive viewer for Chico stalk-and-ball-stick trees.

pub mod camera;
pub mod checkerboard_material;
pub mod commands;
mod ground;
mod render;
mod render_materials;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use render::{RenderConfig, RenderSubject};

use bevy::prelude::*;
use chico_ball_components::frond::FrondRenderItemPlugin;
use chico_ball_components::tuft::render_item_plugin::TuftRenderItemPlugin;
use chico_sbs_trees::braid_oak_tree::render_item_plugin::ensure_registered as ensure_braid_oak_tree_render_plugins;
use chico_sbs_trees::date_palm::render_item_plugin::ensure_registered as ensure_date_palm_render_plugins;
use chico_sbs_trees::friends_conifer::render_item_plugin::ensure_registered as ensure_friends_conifer_render_plugins;
use chico_sbs_trees::honu_banyan::render_item_plugin::ensure_registered as ensure_honu_banyan_render_plugins;
use chico_sbs_trees::jungle_storybook_tree::render_item_plugin::ensure_registered as ensure_jungle_storybook_tree_render_plugins;
use chico_sbs_trees::kamakura_torch::render_item_plugin::ensure_registered as ensure_kamakura_torch_render_plugins;
use chico_sbs_trees::liams_conifer::render_item_plugin::ensure_registered as ensure_liams_conifer_render_plugins;
use chico_sbs_trees::northern_conifer::render_item_plugin::ensure_registered as ensure_northern_conifer_render_plugins;
use chico_sbs_trees::palm_bush::render_item_plugin::ensure_registered as ensure_palm_bush_render_plugins;
use chico_sbs_trees::penmarch_torch::render_item_plugin::ensure_registered as ensure_penmarch_torch_render_plugins;
use chico_sbs_trees::rorys_head_trained::render_item_plugin::ensure_registered as ensure_rorys_head_trained_render_plugins;
use chico_sbs_trees::sopes_banyan::render_item_plugin::ensure_registered as ensure_sopes_banyan_render_plugins;
use chico_sbs_trees::storybook_tree::render_item_plugin::ensure_registered as ensure_storybook_tree_render_plugins;
use chico_sbs_trees::temperate_conifer::render_item_plugin::ensure_registered as ensure_temperate_conifer_render_plugins;
use chico_sbs_trees::vase_tree::render_item_plugin::ensure_registered as ensure_vase_tree_render_plugins;
use chico_sbs_trees::waialea_palm::render_item_plugin::ensure_registered as ensure_waialea_palm_render_plugins;
use chico_sdf::{CrookCylinder, NoisyBall, NoisyCylinder};
use chico_vegetation_shaders::{
	ChicoLeafMaterial, ChicoStickMaterial, ChicoVegetationShadersPlugin,
};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use ground::setup_ground;
use render::sync_render;
use render_item::mesh::handle::EnforceCachingPlugin;
use render_materials::{setup_render_materials, sync_render_material_handles};

pub struct SbsTreesPlaygroundPlugin;

impl Plugin for SbsTreesPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<RenderConfig>();
		ensure_sopes_banyan_render_plugins(app);
		ensure_honu_banyan_render_plugins(app);
		ensure_liams_conifer_render_plugins(app);
		ensure_friends_conifer_render_plugins(app);
		ensure_northern_conifer_render_plugins(app);
		ensure_temperate_conifer_render_plugins(app);
		ensure_date_palm_render_plugins(app);
		ensure_palm_bush_render_plugins(app);
		ensure_waialea_palm_render_plugins(app);
		ensure_storybook_tree_render_plugins(app);
		ensure_penmarch_torch_render_plugins(app);
		ensure_kamakura_torch_render_plugins(app);
		ensure_rorys_head_trained_render_plugins(app);
		ensure_vase_tree_render_plugins(app);
		ensure_braid_oak_tree_render_plugins(app);
		ensure_jungle_storybook_tree_render_plugins(app);
		if !app.is_plugin_added::<TuftRenderItemPlugin>() {
			app.add_plugins(TuftRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<FrondRenderItemPlugin>() {
			app.add_plugins(FrondRenderItemPlugin::default());
		}
		app.add_plugins(ChicoVegetationShadersPlugin);
		ensure_enforce_caching_plugin::<NoisyCylinder, ChicoStickMaterial>(app);
		ensure_enforce_caching_plugin::<CrookCylinder, ChicoStickMaterial>(app);
		ensure_enforce_caching_plugin::<NoisyBall, ChicoLeafMaterial>(app);
		ensure_enforce_caching_plugin::<NoisyBall, ChicoStickMaterial>(app);
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
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
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
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
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadows_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadows_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}
