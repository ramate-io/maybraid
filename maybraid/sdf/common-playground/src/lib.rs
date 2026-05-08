//! Interactive viewer for [`sdf_common::SdfCommonPrimitive`] via marching-cubes meshing.

pub mod camera;
mod ground;
mod input;
mod preview;
mod ui;

pub use camera::CameraController;
pub use preview::{PreviewConfig, SdfPreviewRoot};

use bevy::prelude::*;
use preview::{keyboard_preview, sync_sdf_preview};
use render_item::{mesh::fetch_meshes, mesh::handle::MeshHandle, render_items};
use sdf_common::{SdfCommonPrimitive, SdfCommonRenderItem};

/// Brown-ish default material for SDF previews (similar stick/trunk tone in objects playground).
#[derive(Resource, Clone)]
pub struct PlaygroundMaterial(pub Handle<StandardMaterial>);

pub struct SdfCommonPlaygroundPlugin {
	#[allow(dead_code)]
	pub seed: u32,
}

impl Plugin for SdfCommonPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(PreviewConfig {
				primitive: SdfCommonPrimitive::tapered_cylinder_default(),
				res_2: 4,
			})
			.init_resource::<input::TypedSdfName>()
			.init_resource::<input::TextEntryFocus>()
			.add_systems(
				Startup,
				(
					camera::setup_camera,
					setup_lighting,
					ground::setup_ground,
					setup_preview_material,
					ui::setup_debug_ui,
				),
			)
			.add_systems(
				Update,
				(
					camera::camera_controller,
					keyboard_preview,
					input::toggle_text_entry_focus,
					input::capture_sdf_name_input,
					sync_sdf_preview,
					ui::update_debug_ui,
					render_items::<SdfCommonRenderItem<StandardMaterial>>,
					fetch_meshes::<MeshHandle<SdfCommonPrimitive>, StandardMaterial>,
				),
			);
	}
}

fn setup_preview_material(
	mut commands: Commands,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let handle = materials.add(StandardMaterial {
		base_color: Color::srgb(0.89, 0.886, 0.604),
		..default()
	});
	commands.insert_resource(PlaygroundMaterial(handle));
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
