use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use chunk::cascade::ResolutionMap;
use render_item::lod::LodPlugin;
use render_item::{
	mesh::{
		cache::handle::map::HandleMap, handle::MeshHandle, IdentifiedMesh, MeshBuilder,
		MeshDispatch, MeshId,
	},
	NormalizeChunk, RenderItem,
};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

/// Noisy sphere: a sphere with Perlin noise perturbation for organic surface variation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OceanMesh {}

impl Default for OceanMesh {
	fn default() -> Self {
		Self::new()
	}
}

impl OceanMesh {
	pub fn new() -> Self {
		Self {}
	}

	pub fn y_length(&self, cascade_chunk: &CascadeChunk) -> f32 {
		let y_length = -cascade_chunk.origin.y;

		if y_length < 0.0 {
			return 0.0;
		}

		if y_length > cascade_chunk.size {
			return cascade_chunk.size;
		}

		y_length
	}
}

impl NormalizeChunk for OceanMesh {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		cascade_chunk.clone()
	}
}

impl IdentifiedMesh for OceanMesh {
	fn id(&self) -> MeshId {
		let debug_string = format!("{:?}", self);
		MeshId::new(debug_string)
	}
}

impl MeshBuilder for OceanMesh {
	fn build_mesh_impl(&self, cascade_chunk: &CascadeChunk) -> Option<Mesh> {
		// cuboid over the entire cascade chunk where height is less than
		let y_length = self.y_length(cascade_chunk);

		if y_length == 0.0 {
			return None;
		}

		let mesh = Mesh::from(Cuboid::new(cascade_chunk.size, y_length, cascade_chunk.size));
		Some(mesh)
	}
}

#[derive(Component, Clone)]
pub struct Ocean<T: Material> {
	mesh: OceanMesh,
	material: MeshMaterial3d<T>,
	ocean_cache: HandleMap<OceanMesh>,
}

impl<T: Material> Ocean<T> {
	pub fn new(material: MeshMaterial3d<T>) -> Self {
		Self { mesh: OceanMesh::new(), material, ocean_cache: HandleMap::new() }
	}
}

impl<T: Material> RenderItem for Ocean<T> {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		_transform: Transform,
	) -> Vec<Entity> {
		log::info!("Spawning ocean render items for cascade chunk: {:?}", cascade_chunk);
		let mut render_items = Vec::new();

		let mesh_handle =
			MeshHandle::new(self.mesh.clone()).with_handle_cache(self.ocean_cache.clone());

		let half_height = self.mesh.y_length(cascade_chunk) / 2.0;
		let half_size = cascade_chunk.size / 2.0;
		let transform = Transform::from_translation(
			cascade_chunk.origin + Vec3::new(half_size, half_height, half_size),
		);

		render_items.push(
			commands
				.spawn((
					cascade_chunk.clone(),
					MeshDispatch::new(mesh_handle),
					transform,
					MeshMaterial3d(self.material.0.clone()),
				))
				.id(),
		);
		render_items
	}
}

#[derive(Debug, Clone)]
pub struct OceanPlugin<R: ResolutionMap + Send + Sync + 'static, M: Material> {
	__marker: PhantomData<(R, M)>,
}

impl<R: ResolutionMap + Send + Sync + 'static, M: Material> Default for OceanPlugin<R, M> {
	fn default() -> Self {
		Self { __marker: PhantomData }
	}
}

impl<R: ResolutionMap + Send + Sync + 'static, M: Material> Plugin for OceanPlugin<R, M>
where
	M::Data: PartialEq + Eq + Hash + Clone,
{
	fn build(&self, app: &mut App) {
		app.add_plugins(LodPlugin::<R, Ocean<M>>::default());
	}
}
