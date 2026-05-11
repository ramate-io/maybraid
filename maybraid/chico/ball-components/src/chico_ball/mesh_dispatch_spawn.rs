//! [`StandardMaterial`] handle and [`Command`](bevy::ecs::world::Command) for spawning [`MeshDispatch`](render_item::mesh::MeshDispatch) balls.

use bevy::ecs::world::Command;
use bevy::prelude::*;
use render_item::{
	mesh::{handle::MeshHandle, MeshDispatch},
	CascadeChunk,
};

use super::ChicoBall;

/// Default PBR material for [`MeshDispatch`](render_item::mesh::MeshDispatch) on [`ChicoBall`] entities.
#[derive(Resource, Clone)]
pub(crate) struct ChicoBallDispatchMaterial(pub Handle<StandardMaterial>);

pub(crate) fn init_chico_ball_dispatch_material(
	mut commands: Commands,
	mut mats: ResMut<Assets<StandardMaterial>>,
	existing: Option<Res<ChicoBallDispatchMaterial>>,
) {
	if existing.is_none() {
		commands.insert_resource(ChicoBallDispatchMaterial(mats.add(StandardMaterial::default())));
	}
}

pub(crate) struct SpawnChicoBallMeshCommand {
	pub ball: ChicoBall,
	pub chunk: CascadeChunk,
	pub transform: Transform,
}

impl Command for SpawnChicoBallMeshCommand {
	fn apply(self, world: &mut World) {
		let Some(mat) = world.get_resource::<ChicoBallDispatchMaterial>() else {
			log::warn!(
				"ChicoBallDispatchMaterial missing; add ChicoBallRenderItemPlugin before spawning ChicoBall meshes"
			);
			return;
		};
		let mesh_handle = MeshHandle::new(self.ball.noisy_ball());
		world.spawn((
			self.ball,
			self.chunk,
			MeshDispatch::new(mesh_handle),
			self.transform,
			MeshMaterial3d(mat.0.clone()),
		));
	}
}
