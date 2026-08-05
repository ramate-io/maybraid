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

	pub fn scene_ref(self) -> scene_ref::SceneRef {
		scene_ref::SceneRef::glb(self.0)
	}
}

impl std::fmt::Display for AssetPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

/// Stick GLBs under `vegetation/sticks/`.
pub mod sticks {
	use super::AssetPath;

	macro_rules! stick_lod_triad {
		($mod:ident, $dir:literal) => {
			pub mod $mod {
				use super::AssetPath;

				pub const HIGH: AssetPath =
					AssetPath::new(concat!("vegetation/sticks/", $dir, "/001_high_res.glb"));
				pub const MID: AssetPath =
					AssetPath::new(concat!("vegetation/sticks/", $dir, "/001_mid_res.glb"));
				pub const LOW: AssetPath =
					AssetPath::new(concat!("vegetation/sticks/", $dir, "/001_low_res.glb"));
			}
		};
	}

	stick_lod_triad!(standard, "standard");
	stick_lod_triad!(standard_trunk, "standard_trunk");
}
