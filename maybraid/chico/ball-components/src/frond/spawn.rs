//! Shared spawn path for merged frond [`RenderItem`] types.

use bevy::prelude::*;
use render_item::CascadeChunk;

use crate::merged_mesh::spawn_merged_mesh;

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
		self.spawn_render_items_under(commands, cascade_chunk, transform, None)
	}

	fn spawn_render_items_under(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		local_transform: Transform,
		parent: Option<Entity>,
	) -> Vec<Entity> {
		let mesh = self.build_mesh(1.0);
		let material = self.material_slot();
		vec![spawn_merged_mesh(
			self,
			mesh,
			material,
			commands,
			cascade_chunk,
			local_transform,
			parent,
		)]
	}
}
