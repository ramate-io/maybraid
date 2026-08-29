//! [`SceneRef`] → [`ScenePrototype`] cache.
//!
//! A prototype is kit-local geometry: mesh handles plus local transforms. World
//! pose and [`material_ref::MaterialRef`] are **not** in the key — the renderer
//! combines those only at submit.

use bevy::asset::{AssetServer, Handle};
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::math::Affine3A;
use bevy::mesh::Mesh;
use bevy::platform::collections::HashMap;
use bevy::prelude::{Assets, ChildOf, Entity, Mesh3d, Resource, Transform, World};
use bevy::world_serialization::WorldAsset;

use crate::handles::SceneRefHandles;
use crate::scene_ref::SceneRef;

/// One mesh and its transform inside a compiled [`SceneRef`].
#[derive(Debug, Clone)]
pub struct ScenePrototypePart {
	pub mesh: Handle<Mesh>,
	pub local_transform: Affine3A,
}

/// Compiled kit geometry for one [`SceneRef`]. Shared across every posed instance.
#[derive(Debug, Clone, Default)]
pub struct ScenePrototype {
	pub parts: Vec<ScenePrototypePart>,
}

impl ScenePrototype {
	pub fn is_empty(&self) -> bool {
		self.parts.is_empty()
	}

	/// Walk `world` for [`Mesh3d`] entities and compose parent [`Transform`]s.
	pub fn from_world(world: &World) -> Option<Self> {
		let mut parts = Vec::new();
		for entity in world.iter_entities() {
			let id = entity.id();
			let Some(mesh) = world.get::<Mesh3d>(id) else {
				continue;
			};
			parts.push(ScenePrototypePart {
				mesh: mesh.0.clone(),
				local_transform: composed_affine(world, id),
			});
		}
		if parts.is_empty() {
			None
		} else {
			Some(Self { parts })
		}
	}
}

fn composed_affine(world: &World, entity: Entity) -> Affine3A {
	let local = world
		.get::<Transform>(entity)
		.map(transform_affine)
		.unwrap_or(Affine3A::IDENTITY);
	match world.get::<ChildOf>(entity) {
		Some(parent) => composed_affine(world, parent.parent()) * local,
		None => local,
	}
}

fn transform_affine(transform: &Transform) -> Affine3A {
	Affine3A::from_scale_rotation_translation(
		transform.scale,
		transform.rotation,
		transform.translation,
	)
}

/// Memoized [`ScenePrototype`]s keyed by [`SceneRef`] (path + mirror + reflect).
#[derive(Resource, Default)]
pub struct ScenePrototypeCache {
	cache: HashMap<SceneRef, ScenePrototype>,
}

impl ScenePrototypeCache {
	/// Resolve `scene_ref` to a compiled prototype when the source world is ready.
	///
	/// Misses that are still loading return [`None`] without caching. Cache hits
	/// do not touch [`Assets<Mesh>`].
	pub fn try_resolve(
		&mut self,
		scene_ref: &SceneRef,
		scene_handles: &mut SceneRefHandles,
		asset_server: &AssetServer,
		world_assets: &mut Assets<WorldAsset>,
		meshes: &mut Assets<Mesh>,
		type_registry: &AppTypeRegistry,
	) -> Option<&ScenePrototype> {
		if self.cache.contains_key(scene_ref) {
			return self.cache.get(scene_ref);
		}
		let handle = scene_handles.try_resolve(
			scene_ref,
			asset_server,
			world_assets,
			meshes,
			type_registry,
		)?;
		if !asset_server.is_loaded_with_dependencies(&handle) {
			return None;
		}
		let asset = world_assets.get(&handle)?;
		// Loaded worlds with no Mesh3d cache empty so submit stops retrying.
		let prototype = ScenePrototype::from_world(&asset.world).unwrap_or_default();
		self.cache.insert(scene_ref.clone(), prototype);
		self.cache.get(scene_ref)
	}

	/// Kick off source loads so later [`Self::try_resolve`] hits warm assets.
	pub fn preload<'a>(
		&mut self,
		scene_refs: impl IntoIterator<Item = &'a SceneRef>,
		scene_handles: &mut SceneRefHandles,
		asset_server: &AssetServer,
	) {
		scene_handles.preload(scene_refs, asset_server);
	}

	pub fn len(&self) -> usize {
		self.cache.len()
	}

	pub fn is_empty(&self) -> bool {
		self.cache.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::PrimitiveTopology;

	#[test]
	fn from_world_collects_posed_mesh() {
		let mut meshes = Assets::<Mesh>::default();
		let mesh = meshes.add(Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
		));
		let mut world = World::new();
		let parent = world.spawn(Transform::from_xyz(1.0, 0.0, 0.0)).id();
		world.spawn((Mesh3d(mesh), Transform::from_xyz(0.0, 2.0, 0.0), ChildOf(parent)));

		let proto = ScenePrototype::from_world(&world).expect("mesh part");
		assert_eq!(proto.parts.len(), 1);
		let t = proto.parts[0].local_transform;
		assert!((t.translation.x - 1.0).abs() < 1e-5);
		assert!((t.translation.y - 2.0).abs() < 1e-5);
	}

	#[test]
	fn from_world_empty_without_mesh() {
		let world = World::new();
		assert!(ScenePrototype::from_world(&world).is_none());
	}
}
