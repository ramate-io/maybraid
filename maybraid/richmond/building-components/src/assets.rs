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

/// Urban partition GLBs under `urban/partitions/`.
pub mod partitions {
	use super::AssetPath;

	pub mod rough_stonework {
		use super::AssetPath;

		/// Straight / linear rough stonework segment.
		pub const LINEAR: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_001.glb");
		pub const LINEAR_HIGH: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_001_high_res.glb");
		pub const LINEAR_MID: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_001_mid_res.glb");
		pub const LINEAR_LOW: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_001_low_res.glb");

		/// 180° arc rough stonework.
		pub const ARC_180: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_180_001.glb");
		pub const ARC_180_HIGH: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_180_001_high_res.glb");
		pub const ARC_180_MID: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_180_001_mid_res.glb");
		pub const ARC_180_LOW: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_180_001_low_res.glb");

		/// 90° arc rough stonework.
		pub const ARC_90: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_90_001.glb");
		pub const ARC_90_HIGH: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_90_001_high_res.glb");
		pub const ARC_90_MID: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_90_001_mid_res.glb");
		pub const ARC_90_LOW: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_90_001_low_res.glb");

		/// 15° arc rough stonework.
		pub const ARC_15: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_15_001.glb");
		pub const ARC_15_HIGH: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_15_001_high_res.glb");
		pub const ARC_15_MID: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_15_001_mid_res.glb");
		pub const ARC_15_LOW: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_15_001_low_res.glb");

		/// 90° header rough stonework.
		pub const HEADER_90: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_90_header_001.glb");
		pub const HEADER_90_HIGH: AssetPath = AssetPath::new(
			"urban/partitions/rough_stonework/rough_stonework_90_header_001_high_res.glb",
		);
		pub const HEADER_90_MID: AssetPath = AssetPath::new(
			"urban/partitions/rough_stonework/rough_stonework_90_header_001_mid_res.glb",
		);
		pub const HEADER_90_LOW: AssetPath = AssetPath::new(
			"urban/partitions/rough_stonework/rough_stonework_90_header_001_low_res.glb",
		);

		/// 15° header rough stonework.
		pub const HEADER_15: AssetPath =
			AssetPath::new("urban/partitions/rough_stonework/rough_stonework_15_header_001.glb");
		pub const HEADER_15_HIGH: AssetPath = AssetPath::new(
			"urban/partitions/rough_stonework/rough_stonework_15_header_001_high_res.glb",
		);
		pub const HEADER_15_MID: AssetPath = AssetPath::new(
			"urban/partitions/rough_stonework/rough_stonework_15_header_001_mid_res.glb",
		);
		pub const HEADER_15_LOW: AssetPath = AssetPath::new(
			"urban/partitions/rough_stonework/rough_stonework_15_header_001_low_res.glb",
		);
	}
}

/// Urban floor GLBs under `urban/floors/`.
pub mod floors {
	use super::AssetPath;

	pub mod rough_stonework {
		use super::AssetPath;

		/// Unit rectangular floor slab.
		pub const RECTANGLE: AssetPath =
			AssetPath::new("urban/floors/rough_stonework/rough_stonework_001.glb");
		/// Circle−inscribed-square floor cap (four yaws fill a circular ring).
		pub const CIRCLE_INSCRIBED_SQUARE: AssetPath =
			AssetPath::new("urban/floors/rough_stonework/rough_stonework_inscribed_square_001.glb");
	}
}

/// Urban stair GLBs under `urban/stairs/`.
pub mod stairs {
	use super::AssetPath;

	pub mod rough_stonework {
		use super::AssetPath;

		/// Unit tread cube (\(X = Y = Z \in [-1, 1]\), left face −Z).
		pub const TREAD: AssetPath =
			AssetPath::new("urban/stairs/rough_stonework/rough_stonework_tread_001.glb");
	}
}

/// Urban roof GLBs under `urban/roofs/`.
pub mod roofs {
	use super::AssetPath;

	/// Canonical unit right-triangle primitive (no style / LOD variants).
	pub const UNIT_RIGHT_TRIANGLE: AssetPath =
		AssetPath::new("urban/roofs/unit_right_triangle.glb");

	pub mod shepherds_thatch {
		use super::AssetPath;

		pub const RIGHT_TRIANGLE_HIGH: AssetPath =
			AssetPath::new("urban/roofs/shepherds_thatch/right_triangle_001_high_res.glb");
		pub const RIGHT_TRIANGLE_MID: AssetPath =
			AssetPath::new("urban/roofs/shepherds_thatch/right_triangle_001_mid_res.glb");
		pub const RIGHT_TRIANGLE_LOW: AssetPath =
			AssetPath::new("urban/roofs/shepherds_thatch/right_triangle_001_low_res.glb");
	}
}

