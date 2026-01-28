use crate::TerrainSdf;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use render_item::mesh::cache::{handle::map::HandleMap, mesh::disk::DiskMeshCache};
use render_item::mesh::handle::MeshHandle;
use render_item::{mesh::MeshDispatch, RenderItem};

#[derive(Component, Debug, Clone)]
pub struct TerrainRenderItem<M: Material> {
	pub sdf: TerrainSdf,
	pub material: MeshMaterial3d<M>,
	pub handle_map: HandleMap<TerrainSdf>,
	pub mesh_cache: Option<DiskMeshCache<TerrainSdf>>,
}

impl<M: Material> TerrainRenderItem<M> {
	pub fn new(sdf: TerrainSdf, material: MeshMaterial3d<M>) -> Self {
		Self { sdf, material, handle_map: HandleMap::new(), mesh_cache: None }
	}

	pub fn with_handle_map(mut self, handle_map: HandleMap<TerrainSdf>) -> Self {
		self.handle_map = handle_map;
		self
	}

	pub fn with_mesh_cache(mut self, mesh_cache: Option<DiskMeshCache<TerrainSdf>>) -> Self {
		self.mesh_cache = mesh_cache;
		self
	}
}

impl<M: Material> RenderItem for TerrainRenderItem<M> {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		_transform: Transform,
	) -> Vec<Entity> {
		log::debug!("Spawning terrain render items for cascade chunk: {:?}", cascade_chunk);

		let transform = Transform::from_translation(cascade_chunk.origin);
		let mesh_handle = MeshHandle::new(self.sdf.clone())
			.with_handle_cache(self.handle_map.clone())
			.with_mesh_cache(self.mesh_cache.clone());
		commands.spawn((
			cascade_chunk.clone(),
			MeshDispatch::new(mesh_handle),
			transform,
			self.material.clone(),
		));
		vec![]
	}
}
