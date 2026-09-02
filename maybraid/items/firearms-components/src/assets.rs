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

/// Firearm GLBs under `items/guns/{rigs,bodies,barrels,trigger_boxes,grips,stocks,concepts}/`.
pub mod guns {
	use super::AssetPath;

	pub const FIREARM_RIG: AssetPath = AssetPath::new("items/guns/rigs/firearm_rig.glb");
	pub const BULLPUP_BARREL: AssetPath = AssetPath::new("items/guns/barrels/bullpup_barrel.glb");
	pub const BULLPUP_BODY: AssetPath = AssetPath::new("items/guns/bodies/bullpup_body.glb");
	pub const BULLPUP_FULL_CONCEPT: AssetPath =
		AssetPath::new("items/guns/concepts/bullpup_full_concept.glb");
	pub const BUMP_HANDLE: AssetPath = AssetPath::new("items/guns/grips/bump_handle.glb");
	pub const KEELRIPE_BOX: AssetPath = AssetPath::new("items/guns/trigger_boxes/keelripe_box.glb");
	pub const LAZNARD_BARREL: AssetPath = AssetPath::new("items/guns/barrels/laznard_barrel.glb");
	pub const PADDLE_BOX: AssetPath = AssetPath::new("items/guns/trigger_boxes/paddle_box.glb");
	pub const RELTOR_BODY: AssetPath = AssetPath::new("items/guns/bodies/reltor_body.glb");
	pub const RELTOR_BOX: AssetPath = AssetPath::new("items/guns/trigger_boxes/reltor_box.glb");
	pub const SAMSONIST_BODY: AssetPath = AssetPath::new("items/guns/bodies/samsonist_body.glb");
	pub const SILOPUP_BODY: AssetPath = AssetPath::new("items/guns/bodies/silopup_body.glb");
	pub const SILOPUP_FULL_CONCEPT: AssetPath =
		AssetPath::new("items/guns/concepts/silopup_full_concept.glb");
	pub const SNAILER_BODY: AssetPath = AssetPath::new("items/guns/bodies/snailer_body.glb");
}

/// Melee GLBs under `items/melee/{blades,guards,handles}/` (until a melee crate exists).
pub mod melee {
	use super::AssetPath;

	pub const LICUCIAN_BLADE: AssetPath = AssetPath::new("items/melee/blades/licucian_blade.glb");
	pub const LICUCIAN_GUARD: AssetPath = AssetPath::new("items/melee/guards/licucian_guard.glb");
	pub const LICUCIAN_HANDLE: AssetPath =
		AssetPath::new("items/melee/handles/licucian_handle.glb");
}
