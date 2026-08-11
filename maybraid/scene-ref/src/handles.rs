//! Memoized [`Handle<WorldAsset>`]s keyed by [`SceneRef`].

use bevy::asset::{AssetServer, Handle};
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::mesh::Mesh;
use bevy::platform::collections::HashMap;
use bevy::prelude::{Assets, Resource};
use bevy::world_serialization::WorldAsset;

use crate::mirror::mirror_world_asset;
use crate::scene_ref::SceneRef;

/// Memoized [`Handle<WorldAsset>`]s keyed by [`SceneRef`] (path + mirror).
#[derive(Resource, Default)]
pub struct SceneRefHandles {
	cache: HashMap<SceneRef, Handle<WorldAsset>>,
}

impl SceneRefHandles {
	fn ensure_unmirrored(
		&mut self,
		scene_ref: &SceneRef,
		asset_server: &AssetServer,
	) -> Handle<WorldAsset> {
		debug_assert!(scene_ref.mirror.is_none());
		if let Some(handle) = self.cache.get(scene_ref) {
			return handle.clone();
		}
		let handle = scene_ref.load_source(asset_server);
		self.cache.insert(scene_ref.clone(), handle.clone());
		handle
	}

	/// Return a strong handle for an **unmirrored** `scene_ref`, loading once per path.
	///
	/// Mirrored refs must go through [`Self::try_resolve`] (or [`SceneRefRoot`] fulfill).
	pub fn handle(
		&mut self,
		scene_ref: &SceneRef,
		asset_server: &AssetServer,
	) -> Handle<WorldAsset> {
		assert!(
			scene_ref.mirror.is_none(),
			"SceneRefHandles::handle is for unmirrored refs; use try_resolve for mirrors"
		);
		self.ensure_unmirrored(scene_ref, asset_server)
	}

	/// Resolve `scene_ref` to a cached handle when ready.
	///
	/// Unmirrored refs always return a (possibly still-loading) handle.
	/// Mirrored refs return [`None`] until the source is
	/// [`AssetServer::is_loaded_with_dependencies`] and the rebuilt world is cached.
	pub fn try_resolve(
		&mut self,
		scene_ref: &SceneRef,
		asset_server: &AssetServer,
		world_assets: &mut Assets<WorldAsset>,
		meshes: &mut Assets<Mesh>,
		type_registry: &AppTypeRegistry,
	) -> Option<Handle<WorldAsset>> {
		if let Some(handle) = self.cache.get(scene_ref) {
			return Some(handle.clone());
		}

		match scene_ref.mirror {
			None => Some(self.ensure_unmirrored(scene_ref, asset_server)),
			Some(axis) => {
				let source_handle =
					self.ensure_unmirrored(&scene_ref.without_mirror(), asset_server);
				if !asset_server.is_loaded_with_dependencies(&source_handle) {
					return None;
				}
				let source = world_assets.get(&source_handle)?;
				let mirrored = mirror_world_asset(source, axis, meshes, type_registry)?;
				let handle = world_assets.add(mirrored);
				self.cache.insert(scene_ref.clone(), handle.clone());
				Some(handle)
			}
		}
	}

	/// Preload many refs (e.g. at startup) so later scene spawns hit the cache.
	///
	/// Mirrored refs only kick off their source load; the mirrored rebuild still
	/// needs [`Self::try_resolve`] once dependencies are ready.
	pub fn preload<'a>(
		&mut self,
		scene_refs: impl IntoIterator<Item = &'a SceneRef>,
		asset_server: &AssetServer,
	) {
		for scene_ref in scene_refs {
			let _ = self.ensure_unmirrored(&scene_ref.without_mirror(), asset_server);
		}
	}

	/// Number of cached handles (source and mirrored).
	pub fn len(&self) -> usize {
		self.cache.len()
	}

	pub fn is_empty(&self) -> bool {
		self.cache.is_empty()
	}
}
