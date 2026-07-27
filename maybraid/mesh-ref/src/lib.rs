//! Mesh loading via references to avoid loading the same mesh multiple times.
//!
//! [`MeshRef::Glb`] names a GLB (optionally with a `#SceneN` label). [`MeshRefPlugin`]
//! installs [`MeshRefHandles`], which memoizes [`Handle<WorldAsset>`]s so repeated
//! scene spawns share one load. BSN can pass a [`MeshRef`] wherever a
//! [`HandleTemplate<WorldAsset>`] is expected (`WorldAssetRoot`), or use
//! [`MeshRef::scene`].

use std::path::Path;

use bevy::asset::{AssetPath, AssetServer, Handle, HandleTemplate};
use bevy::platform::collections::HashMap;
use bevy::prelude::{App, Plugin, Resource};
use bevy::scene::prelude::bsn;
use bevy::scene::Scene;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

/// Reference to a mesh / scene asset that can be resolved to a shared handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MeshRef {
	/// Path to a `.glb` (or other glTF) asset. If the path has no `#` label,
	/// scene `0` is used (`path#Scene0`).
	Glb(String),
}

impl MeshRef {
	/// GLB / glTF at `path` (scene 0 unless `path` already includes a label).
	pub fn glb(path: impl Into<String>) -> Self {
		Self::Glb(path.into())
	}

	/// Asset path string used for loading (includes `#Scene0` when unlabeled).
	pub fn labeled_path(&self) -> String {
		match self {
			Self::Glb(path) if path.contains('#') => path.clone(),
			Self::Glb(path) => format!("{path}#Scene0"),
		}
	}

	/// Load (or reuse via [`AssetServer`]) the [`WorldAsset`] for this ref.
	pub fn load(&self, asset_server: &AssetServer) -> Handle<WorldAsset> {
		asset_server.load(self.labeled_path())
	}

	/// BSN scene root for this mesh (`WorldAssetRoot` + shared handle path).
	pub fn scene(self) -> impl Scene + 'static {
		bsn! {
			WorldAssetRoot({self})
		}
	}
}

impl From<&str> for MeshRef {
	fn from(path: &str) -> Self {
		Self::glb(path)
	}
}

impl From<String> for MeshRef {
	fn from(path: String) -> Self {
		Self::Glb(path)
	}
}

impl From<&Path> for MeshRef {
	fn from(path: &Path) -> Self {
		Self::glb(path.to_string_lossy())
	}
}

impl From<MeshRef> for HandleTemplate<WorldAsset> {
	fn from(mesh_ref: MeshRef) -> Self {
		// Path form: AssetServer dedupes loads; MeshRefHandles keeps strong handles.
		HandleTemplate::Path(AssetPath::from(mesh_ref.labeled_path()))
	}
}

impl From<&MeshRef> for HandleTemplate<WorldAsset> {
	fn from(mesh_ref: &MeshRef) -> Self {
		HandleTemplate::Path(AssetPath::from(mesh_ref.labeled_path()))
	}
}

/// Memoized [`Handle<WorldAsset>`]s keyed by [`MeshRef`].
#[derive(Resource, Default)]
pub struct MeshRefHandles {
	cache: HashMap<MeshRef, Handle<WorldAsset>>,
}

impl MeshRefHandles {
	/// Return a strong handle for `mesh_ref`, loading once per distinct ref.
	pub fn handle(&mut self, mesh_ref: &MeshRef, asset_server: &AssetServer) -> Handle<WorldAsset> {
		if let Some(handle) = self.cache.get(mesh_ref) {
			return handle.clone();
		}
		let handle = mesh_ref.load(asset_server);
		self.cache.insert(mesh_ref.clone(), handle.clone());
		handle
	}

	/// Preload many refs (e.g. at startup) so later scene spawns hit the cache.
	pub fn preload<'a>(
		&mut self,
		mesh_refs: impl IntoIterator<Item = &'a MeshRef>,
		asset_server: &AssetServer,
	) {
		for mesh_ref in mesh_refs {
			let _ = self.handle(mesh_ref, asset_server);
		}
	}

	/// Number of cached handles.
	pub fn len(&self) -> usize {
		self.cache.len()
	}

	pub fn is_empty(&self) -> bool {
		self.cache.is_empty()
	}
}

/// Installs [`MeshRefHandles`] for shared GLB / glTF scene loading.
pub struct MeshRefPlugin;

impl Plugin for MeshRefPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<MeshRefHandles>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unlabeled_glb_gets_scene0() -> anyhow::Result<()> {
		let m = MeshRef::glb("urban/foo.glb");
		assert_eq!(m.labeled_path(), "urban/foo.glb#Scene0");
		Ok(())
	}

	#[test]
	fn labeled_path_preserved() -> anyhow::Result<()> {
		let m = MeshRef::glb("urban/foo.glb#Scene1");
		assert_eq!(m.labeled_path(), "urban/foo.glb#Scene1");
		Ok(())
	}
}
