//! Merge multiple [`SceneRef`] worlds into one mesh [`WorldAsset`].

mod mesh;
mod transform_key;

use bevy::asset::{AssetServer, Handle};
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::mesh::Mesh;
use bevy::platform::collections::HashMap;
use bevy::prelude::{Assets, Component, Resource, Transform};
use bevy::scene::prelude::{bsn, template_value};
use bevy::scene::Scene;
use bevy::world_serialization::WorldAsset;

use crate::handles::SceneRefHandles;
use crate::scene_ref::SceneRef;

pub use transform_key::TransformKey;

use mesh::{merge_meshes, merge_world_asset_meshes, world_asset_from_mesh};

/// One input scene and its local transform in the merged multi-scene.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MultiScenePart {
	pub scene: SceneRef,
	pub transform: TransformKey,
}

impl MultiScenePart {
	pub fn new(scene: SceneRef, transform: Transform) -> Self {
		Self { scene, transform: TransformKey::new(transform) }
	}

	pub fn identity(scene: SceneRef) -> Self {
		Self { scene, transform: TransformKey::IDENTITY }
	}
}

/// Ordered list of scene parts to merge into a single-mesh [`WorldAsset`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct MultiSceneMerge {
	pub parts: Vec<MultiScenePart>,
}

/// BSN / ECS root that resolves to [`WorldAssetRoot`] via [`MultiSceneMergeHandles`].
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct MultiSceneMergeRoot(pub MultiSceneMerge);

impl MultiSceneMerge {
	pub fn new(parts: impl IntoIterator<Item = MultiScenePart>) -> Self {
		Self { parts: parts.into_iter().collect() }
	}

	pub fn part(mut self, scene: SceneRef, transform: Transform) -> Self {
		self.parts.push(MultiScenePart::new(scene, transform));
		self
	}

	/// BSN scene root (`MultiSceneMergeRoot`; fulfilled to [`WorldAssetRoot`]).
	pub fn scene(self) -> impl Scene + 'static {
		bsn! {
			template_value(MultiSceneMergeRoot(self))
		}
	}
}

/// Memoized merged [`Handle<WorldAsset>`]s keyed by [`MultiSceneMerge`].
#[derive(Resource, Default)]
pub struct MultiSceneMergeHandles {
	cache: HashMap<MultiSceneMerge, Handle<WorldAsset>>,
}

impl MultiSceneMergeHandles {
	/// Resolve `merge` to a cached handle when every part is ready.
	///
	/// Returns [`None`] until all part [`SceneRef`]s resolve and their mesh
	/// dependencies are loaded. Pipelines through [`SceneRefHandles`] so mirrors
	/// rebuild before merge.
	pub fn try_resolve(
		&mut self,
		merge: &MultiSceneMerge,
		scene_handles: &mut SceneRefHandles,
		asset_server: &AssetServer,
		world_assets: &mut Assets<WorldAsset>,
		meshes: &mut Assets<Mesh>,
		type_registry: &AppTypeRegistry,
	) -> Option<Handle<WorldAsset>> {
		if let Some(handle) = self.cache.get(merge) {
			return Some(handle.clone());
		}

		let mut part_handles = Vec::with_capacity(merge.parts.len());
		for part in &merge.parts {
			let handle = scene_handles.try_resolve(
				&part.scene,
				asset_server,
				world_assets,
				meshes,
				type_registry,
			)?;
			if !asset_server.is_loaded_with_dependencies(&handle) {
				return None;
			}
			part_handles.push((handle, part.transform.0));
		}

		let mut prepared = Vec::new();
		for (handle, part_transform) in &part_handles {
			let source = world_assets.get(handle)?;
			if let Some(part_mesh) = merge_world_asset_meshes(source, *part_transform, meshes) {
				prepared.push(part_mesh);
			}
		}

		// Empty merge (no Mesh3d in any part) still caches so fulfill does not spin.
		let merged_mesh = match merge_meshes(prepared) {
			Some(mesh) => mesh,
			None => {
				use bevy::asset::RenderAssetUsages;
				use bevy::mesh::PrimitiveTopology;
				Mesh::new(
					PrimitiveTopology::TriangleList,
					RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
				)
			}
		};
		let mesh_handle = meshes.add(merged_mesh);
		let world_asset = world_asset_from_mesh(mesh_handle);
		let handle = world_assets.add(world_asset);
		self.cache.insert(merge.clone(), handle.clone());
		Some(handle)
	}

	/// Kick off source loads for every part (mirrored rebuild / merge still need
	/// [`Self::try_resolve`] once dependencies are ready).
	pub fn preload(
		&mut self,
		merge: &MultiSceneMerge,
		scene_handles: &mut SceneRefHandles,
		asset_server: &AssetServer,
	) {
		scene_handles.preload(merge.parts.iter().map(|p| &p.scene), asset_server);
	}

	pub fn len(&self) -> usize {
		self.cache.len()
	}

	pub fn is_empty(&self) -> bool {
		self.cache.is_empty()
	}
}
