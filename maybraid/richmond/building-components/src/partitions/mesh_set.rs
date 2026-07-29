//! Shared partition mesh path sets (no banding policy).

use bevy::scene::prelude::Scene;

use crate::assets::AssetPath;

/// Which of the three warm MeshRef children is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionMeshTier {
	Low,
	Mid,
	High,
}

/// High / mid / low GLB set for one kit piece (may repeat the same path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionMeshSet {
	pub high: AssetPath,
	pub mid: AssetPath,
	pub low: AssetPath,
}

impl PartitionMeshSet {
	pub const fn new(high: AssetPath, mid: AssetPath, low: AssetPath) -> Self {
		Self { high, mid, low }
	}

	pub const fn uniform(asset: AssetPath) -> Self {
		Self {
			high: asset,
			mid: asset,
			low: asset,
		}
	}

	pub fn for_tier(self, tier: PartitionMeshTier) -> AssetPath {
		match tier {
			PartitionMeshTier::High => self.high,
			PartitionMeshTier::Mid => self.mid,
			PartitionMeshTier::Low => self.low,
		}
	}
}

pub fn mesh_child(asset: AssetPath) -> Box<dyn Scene> {
	Box::new(asset.mesh_ref().scene())
}
