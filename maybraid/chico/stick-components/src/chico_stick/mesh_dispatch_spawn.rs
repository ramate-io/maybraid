//! [`StandardMaterial`] handle and [`Command`](bevy::ecs::world::Command) for spawning [`MeshDispatch`](render_item::mesh::MeshDispatch) sticks.

use bevy::ecs::world::Command;
use bevy::prelude::*;
use render_item::{
	mesh::{handle::MeshHandle, MeshDispatch},
	CascadeChunk,
};

use super::ChicoStick;

/// Default PBR material for [`MeshDispatch`](render_item::mesh::MeshDispatch) on [`ChicoStick`] entities.
#[derive(Resource, Clone)]
pub(crate) struct ChicoStickDispatchMaterial(pub Handle<StandardMaterial>);

pub(crate) fn init_chico_stick_dispatch_material(
	mut commands: Commands,
	mut mats: ResMut<Assets<StandardMaterial>>,
	existing: Option<Res<ChicoStickDispatchMaterial>>,
) {
	if existing.is_none() {
		commands.insert_resource(ChicoStickDispatchMaterial(mats.add(StandardMaterial::default())));
	}
}

pub(crate) struct SpawnChicoStickMeshCommand {
	pub stick: ChicoStick,
	pub chunk: CascadeChunk,
	pub transform: Transform,
}

impl Command for SpawnChicoStickMeshCommand {
	fn apply(self, world: &mut World) {
		let Some(mat) = world.get_resource::<ChicoStickDispatchMaterial>() else {
			log::warn!(
				"ChicoStickDispatchMaterial missing; add ChicoStickRenderItemPlugin before spawning ChicoStick meshes"
			);
			return;
		};
		let mesh_handle = MeshHandle::new(self.stick.noisy_cylinder());
		world.spawn((
			self.stick,
			self.chunk,
			MeshDispatch::new(mesh_handle),
			self.transform,
			MeshMaterial3d(mat.0.clone()),
		));
	}
}
