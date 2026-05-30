//! Shared spawn path for merged frond [`RenderItem`] types.

use bevy::prelude::*;
use render_item::CascadeChunk;

/// Strip non-uniform scale from a spawn transform; return uniform factor for mesh authoring.
pub(crate) trait FrondSpawnTransform {
	fn frond_spawn_uniform(self) -> (Transform, f32);
}

impl FrondSpawnTransform for Transform {
	fn frond_spawn_uniform(self) -> (Transform, f32) {
		let s = self.scale;
		let uniform = s.x.abs().max(s.y.abs()).max(s.z.abs()).max(1e-8);
		(
			Transform {
				translation: self.translation,
				rotation: self.rotation,
				scale: Vec3::ONE,
			},
			uniform,
		)
	}
}

pub(crate) trait MergedFrond: Component + Clone + Send + Sync + 'static {
	type Mat: Material + Send + Sync + 'static;
	type MatSlot: Clone + Into<MeshMaterial3d<Self::Mat>> + Send + Sync + 'static;

	fn material_slot(&self) -> Self::MatSlot;
	fn build_mesh(&self, world_uniform_scale: f32) -> Mesh;

	fn spawn_render_entities(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let item = self.clone();
		let material = self.material_slot();
		let (root_transform, world_uniform_scale) = transform.frond_spawn_uniform();
		let root = commands
			.spawn((
				item.clone(),
				cascade_chunk.clone(),
				root_transform,
				Visibility::default(),
			))
			.id();

		commands.queue(move |world: &mut World| {
			let mesh = item.build_mesh(world_uniform_scale);
			let mesh_handle = {
				let mut meshes = world.resource_mut::<Assets<Mesh>>();
				meshes.add(mesh)
			};
			let mesh_material: MeshMaterial3d<Self::Mat> = material.into();
			world.spawn((ChildOf(root), Mesh3d(mesh_handle), mesh_material, Transform::IDENTITY));
		});

		vec![root]
	}
}
