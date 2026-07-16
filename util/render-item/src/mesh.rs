pub mod cache;
pub mod handle;

use crate::NormalizeChunk;
use bevy::prelude::*;
use cache::{handle::MeshHandleCache, mesh::MeshCache};
use chunk::cascade::CascadeChunk;
use std::hash::Hash;
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MeshId(String);

impl MeshId {
	pub fn new(id: String) -> Self {
		Self(id)
	}

	pub fn with_suffix(&self, suffix: &str) -> Self {
		Self(format!("{}{}", self.0, suffix))
	}
}

pub trait IdentifiedMesh {
	fn id(&self) -> MeshId;
}

pub trait MeshBuilder: Clone + NormalizeChunk {
	/// The actual implementation which builds the mesh.
	fn build_mesh_impl(&self, cascade_chunk: &CascadeChunk) -> Option<Mesh>;

	/// Builds a mesh by normalizing the chunk and then building the mesh.
	fn build_mesh(&self, cascade_chunk: &CascadeChunk) -> Option<Mesh> {
		let normalized_chunk = self.normalize_chunk(cascade_chunk);
		self.build_mesh_impl(&normalized_chunk)
	}
}

pub trait MeshFetcher: Clone + IdentifiedMesh {
	/// Builds mesh if it doesn't exist or fetches from the assets. Returns the handle to the mesh.
	fn fetch_mesh(
		&self,
		meshes: &mut ResMut<Assets<Mesh>>,
		cascade_chunk: &CascadeChunk,
	) -> Option<Handle<Mesh>>;
}

/// If it's already defined how the mesh is built, cached, and fetched, this trait can be used to fetch the mesh.
impl<T: MeshBuilder + MeshCache + MeshHandleCache> MeshFetcher for T {
	fn fetch_mesh(
		&self,
		meshes: &mut ResMut<Assets<Mesh>>,
		cascade_chunk: &CascadeChunk,
	) -> Option<Handle<Mesh>> {
		let normalized_cascade_chunk = self.normalize_chunk(cascade_chunk);

		// Check if the mesh handle is already cached.
		if let Some(mesh) = self.fetch_cached_mesh_handle(&normalized_cascade_chunk) {
			log::debug!("Using cached mesh handle for type: {}", std::any::type_name::<Self>());
			return Some(mesh);
		}

		// Check if the mesh is already cached (this will most often get hit when the mesh is on disk).
		let mesh_handle = if let Some(mesh) = self.fetch_cached_mesh(&normalized_cascade_chunk) {
			log::debug!("Using cached mesh for type: {}", std::any::type_name::<Self>());
			Some(meshes.add(mesh))
		} else {
			self.build_mesh(cascade_chunk).map(|mesh| {
				self.cache_mesh(&mesh, &normalized_cascade_chunk);
				log::debug!("Adding mesh to assets for type: {}", std::any::type_name::<Self>());
				meshes.add(mesh)
			})
		};

		mesh_handle.map(|handle| {
			self.cache_mesh_handle(handle.clone(), &normalized_cascade_chunk);
			log::debug!("Caching mesh handle for type: {}", std::any::type_name::<Self>());
			handle
		})
	}
}

/// A mesh dispatch signals an intent for the item to be spawned into the world.
/// This is set up for asynchronous pipelines
/// wherein the mesh may need to be built, fetched from cache, etc.
#[derive(Component, Clone)]
pub struct MeshDispatch<T: MeshFetcher> {
	fetcher: T,
}

impl<T: MeshFetcher> MeshDispatch<T> {
	pub fn new(fetcher: T) -> Self {
		Self { fetcher }
	}
}

/// Fetches meshes and spawns them into the world.
///
/// TODO: this needs to be made event-based.
pub fn fetch_meshes<T: MeshFetcher + Send + Sync + 'static, M: Material>(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	query: Query<
		(Entity, &MeshDispatch<T>, &CascadeChunk, &Transform, &MeshMaterial3d<M>),
		Added<MeshDispatch<T>>,
	>,
) {
	for (parent_entity, mesh_dispatch, cascade_chunk, _transform, material) in &query {
		if let Some(mesh) = mesh_dispatch.fetcher.fetch_mesh(&mut meshes, cascade_chunk) {
			commands.entity(parent_entity).with_children(|parent| {
				parent.spawn((
					Mesh3d(mesh),
					MeshMaterial3d(material.0.clone()),
					Transform::default(), // local to parent, no extra offset/scale/rotation
				));
			});
		}
	}
}

pub struct MeshDispatchPlugin<T: MeshFetcher + Send + Sync + 'static, M: Material> {
	__marker: PhantomData<(T, M)>,
}

impl<T: MeshFetcher + Send + Sync + 'static, M: Material> Default for MeshDispatchPlugin<T, M> {
	fn default() -> Self {
		Self { __marker: PhantomData }
	}
}

impl<T: MeshFetcher + Send + Sync + 'static, M: Material> Plugin for MeshDispatchPlugin<T, M> {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, fetch_meshes::<T, M>);
	}
}
