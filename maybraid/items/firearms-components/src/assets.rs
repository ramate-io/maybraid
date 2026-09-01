//! Asset paths relative to the `maybraid/assets` root.

/// Runtime asset path relative to the Bevy asset root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetPath(&'static str);

impl AssetPath {
	pub const fn new(path: &'static str) -> Self {
		Self(path)
	}

	pub const fn as_str(self) -> &'static str {
		self.0
	}

	/// GLTF scene label for [`bevy::scene::WorldAssetRoot`] (`path#Scene0`).
	pub fn gltf_scene_0(self) -> String {
		format!("{}#Scene0", self.0)
	}

	/// Shared [`scene_ref::SceneRef`] for this GLB (scene 0).
	pub fn scene_ref(self) -> scene_ref::SceneRef {
		scene_ref::SceneRef::glb(self.0)
	}
}

impl std::fmt::Display for AssetPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

/// Firearm GLBs under `items/guns/`.
pub mod guns {
	use super::AssetPath;

	pub const BULLPUP_BARREL: AssetPath = AssetPath::new("items/guns/bullpup_barrel.glb");
	pub const BULLPUP_BODY: AssetPath = AssetPath::new("items/guns/bullpup_body.glb");
	pub const BULLPUP_FULL_CONCEPT: AssetPath =
		AssetPath::new("items/guns/bullpup_full_concept.glb");
	pub const BULLPUP_GRIP: AssetPath = AssetPath::new("items/guns/bullpup_grip.glb");
	pub const KEELRIPE_BODY: AssetPath = AssetPath::new("items/guns/keelripe_body.glb");
	pub const LAZNARD_BARREL: AssetPath = AssetPath::new("items/guns/laznard_barrel.glb");
	pub const RELTOR_BODY: AssetPath = AssetPath::new("items/guns/reltor_body.glb");
	pub const SAMSONIST_BODY: AssetPath = AssetPath::new("items/guns/samsonist_body.glb");
	pub const SILOPUP_BODY: AssetPath = AssetPath::new("items/guns/silopup_body.glb");
	pub const SILOPUP_FULL_CONCEPT: AssetPath =
		AssetPath::new("items/guns/silopup_full_concept.glb");
	pub const SNAILER_BODY: AssetPath = AssetPath::new("items/guns/snailer_body.glb");
}

/// Melee GLBs under `items/melee/` (catalogued here until a melee crate exists).
pub mod melee {
	use super::AssetPath;

	pub const LICUCIAN_BLADE: AssetPath = AssetPath::new("items/melee/licucian_blade.glb");
	pub const LICUCIAN_GUARD: AssetPath = AssetPath::new("items/melee/licucian_guard.glb");
	pub const LICUCIAN_HANDLE: AssetPath = AssetPath::new("items/melee/licucian_handle.glb");
}
