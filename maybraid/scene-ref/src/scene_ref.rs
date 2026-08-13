//! [`SceneRef`] identity: path, optional [`MirrorAxis`], and BSN root.

use std::path::Path;

use bevy::asset::{AssetPath, AssetServer, Handle, HandleTemplate};
use bevy::prelude::{Component, Vec3};
use bevy::scene::prelude::{bsn, template_value};
use bevy::scene::Scene;
use bevy::world_serialization::WorldAsset;

/// Axis along which a [`SceneRef`] rebuilds mirrored mesh geometry (and, for
/// [`SceneRef::reflected`], instance transforms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MirrorAxis {
	X,
	Y,
	Z,
}

impl MirrorAxis {
	pub(crate) fn scale(self) -> Vec3 {
		match self {
			Self::X => Vec3::new(-1.0, 1.0, 1.0),
			Self::Y => Vec3::new(1.0, -1.0, 1.0),
			Self::Z => Vec3::new(1.0, 1.0, -1.0),
		}
	}
}

/// Reference to a scene asset that can be resolved to a shared handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SceneRef {
	/// Path to a `.glb` (or other glTF) asset. If the path has no `#` label,
	/// scene `0` is used (`path#Scene0`).
	pub path: String,
	/// When set, resolve to a rebuilt [`WorldAsset`] mirrored on this axis.
	pub mirror: Option<MirrorAxis>,
	/// When `mirror` is set, also conjugate instance [`bevy::prelude::Transform`]s
	/// (`S M S`) so the hierarchy matches a parent axis-flip. Vertex-only
	/// [`Self::mirrored`] leaves this false; [`Self::reflected`] sets it.
	pub reflect_instance: bool,
}

/// BSN / ECS root that resolves to [`WorldAssetRoot`] via [`SceneRefHandles`].
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SceneRefRoot(pub SceneRef);

impl SceneRef {
	/// GLB / glTF at `path` (scene 0 unless `path` already includes a label).
	pub fn glb(path: impl Into<String>) -> Self {
		Self { path: path.into(), mirror: None, reflect_instance: false }
	}

	/// Same path with **vertex/winding** mirroring (caller places the instance).
	pub fn mirrored(mut self, axis: MirrorAxis) -> Self {
		self.mirror = Some(axis);
		self.reflect_instance = false;
		self
	}

	/// Same path with vertex/winding mirroring **and** conjugated instance TRS.
	///
	/// Equivalent to a parent `scale(axis)` with positive scale at the caller.
	/// Use this for skinned / hierarchical GLBs (e.g. character features).
	pub fn reflected(mut self, axis: MirrorAxis) -> Self {
		self.mirror = Some(axis);
		self.reflect_instance = true;
		self
	}

	/// Set or clear the mirror axis (vertex/winding only; clears instance reflect).
	pub fn with_mirror(mut self, mirror: Option<MirrorAxis>) -> Self {
		self.mirror = mirror;
		self.reflect_instance = false;
		self
	}

	/// Source (unmirrored) ref sharing this path.
	pub fn without_mirror(&self) -> Self {
		Self { path: self.path.clone(), mirror: None, reflect_instance: false }
	}

	/// Asset path string used for loading the **source** glTF (includes `#Scene0` when unlabeled).
	pub fn labeled_path(&self) -> String {
		if self.path.contains('#') {
			self.path.clone()
		} else {
			format!("{}#Scene0", self.path)
		}
	}

	/// Load (or reuse via [`AssetServer`]) the source [`WorldAsset`] for this path.
	pub fn load_source(&self, asset_server: &AssetServer) -> Handle<WorldAsset> {
		asset_server.load(self.labeled_path())
	}

	/// BSN scene root for this ref (`SceneRefRoot`; fulfilled to [`WorldAssetRoot`]).
	pub fn scene(self) -> impl Scene + 'static {
		bsn! {
			template_value(SceneRefRoot(self))
		}
	}
}

impl From<&str> for SceneRef {
	fn from(path: &str) -> Self {
		Self::glb(path)
	}
}

impl From<String> for SceneRef {
	fn from(path: String) -> Self {
		Self::glb(path)
	}
}

impl From<&Path> for SceneRef {
	fn from(path: &Path) -> Self {
		Self::glb(path.to_string_lossy())
	}
}

impl From<SceneRef> for HandleTemplate<WorldAsset> {
	fn from(scene_ref: SceneRef) -> Self {
		assert!(
			scene_ref.mirror.is_none(),
			"mirrored/reflected SceneRef cannot convert to HandleTemplate::Path; use SceneRef::scene()"
		);
		HandleTemplate::Path(AssetPath::from(scene_ref.labeled_path()))
	}
}

impl From<&SceneRef> for HandleTemplate<WorldAsset> {
	fn from(scene_ref: &SceneRef) -> Self {
		assert!(
			scene_ref.mirror.is_none(),
			"mirrored/reflected SceneRef cannot convert to HandleTemplate::Path; use SceneRef::scene()"
		);
		HandleTemplate::Path(AssetPath::from(scene_ref.labeled_path()))
	}
}
