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
}

impl std::fmt::Display for AssetPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

/// Urban partition GLBs under `urban/partitions/`.
pub mod partitions {
	use super::AssetPath;

	pub mod rough_stonework {
		use super::AssetPath;

		/// Straight / linear rough stonework segment.
		pub const LINEAR: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_001.glb");
		/// 180° arc rough stonework.
		pub const ARC_180: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_180_001.glb");
		/// 90° arc rough stonework.
		pub const ARC_90: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_90_001.glb");
		/// 90° header rough stonework.
		pub const HEADER_90: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_90_header_001.glb");
	}
}
