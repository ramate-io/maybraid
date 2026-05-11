//! Registers [`MeshDispatchPlugin`](render_item::mesh::MeshDispatchPlugin) for [`MeshHandle`](render_item::mesh::handle::MeshHandle)`<`[`NoisyBall`](chico_sdf::NoisyBall)`>` and a default [`StandardMaterial`] for [`super::ChicoBall`] mesh dispatch.

use bevy::prelude::*;
use chico_sdf::NoisyBall;
use render_item::mesh::{handle::MeshHandle, MeshDispatchPlugin};

use super::mesh_dispatch_spawn::init_chico_ball_dispatch_material;

pub struct ChicoBallRenderItemPlugin;

impl Default for ChicoBallRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for ChicoBallRenderItemPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init_chico_ball_dispatch_material);
		if !app.is_plugin_added::<MeshDispatchPlugin<MeshHandle<NoisyBall>, StandardMaterial>>() {
			app.add_plugins(MeshDispatchPlugin::<MeshHandle<NoisyBall>, StandardMaterial>::default());
		}
	}
}
