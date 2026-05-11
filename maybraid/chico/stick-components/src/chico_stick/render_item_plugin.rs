//! Registers [`MeshDispatchPlugin`](render_item::mesh::MeshDispatchPlugin) for [`MeshHandle`](render_item::mesh::handle::MeshHandle)`<`[`NoisyCylinder`](chico_sdf::NoisyCylinder)`>` and a default [`StandardMaterial`] for [`super::ChicoStick`] mesh dispatch.

use bevy::prelude::*;
use chico_sdf::NoisyCylinder;
use render_item::mesh::{handle::MeshHandle, MeshDispatchPlugin};

use super::mesh_dispatch_spawn::init_chico_stick_dispatch_material;

pub struct ChicoStickRenderItemPlugin;

impl Default for ChicoStickRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for ChicoStickRenderItemPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_chico_stick_dispatch_material);
		if !app.is_plugin_added::<MeshDispatchPlugin<MeshHandle<NoisyCylinder>, StandardMaterial>>() {
			app.add_plugins(MeshDispatchPlugin::<MeshHandle<NoisyCylinder>, StandardMaterial>::default());
		}
	}
}
