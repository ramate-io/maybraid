//! Circular joint between partition segments (\(X,Z \in [-0.5, 0.5]\), \(Y \in [0, 1]\)).
//!
//! High + mid GLBs only; low / ultra-low LOD omit this filler.

use crate::partitions::geometry::JointLod;
use crate::partitions::probe::{leaf_partition_lod_level, leaf_partition_lod_status};

/// Circular / post joint between upright linear partition segments.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkJoint;

impl lod::gen::LodScene for RoughStoneworkJoint {
	fn scene_lod_level(&self, lod_ref: &lod::lod_ref::LodRef) -> lod::gen::LodSceneLevel {
		leaf_partition_lod_level(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &lod::lod_ref::LodRef) -> lod::gen::LodSceneStatus {
		leaf_partition_lod_status(lod_ref)
	}

	fn scene_with_level(
		&self,
		_lod_ref: &lod::lod_ref::LodRef,
		level: lod::gen::LodSceneLevel,
	) -> impl bevy::scene::Scene + 'static {
		JointLod::posed_tier(bevy::prelude::Transform::IDENTITY, level)
	}

	fn scene_with_lod(&self, lod_ref: &lod::lod_ref::LodRef) -> impl bevy::scene::Scene + 'static {
		JointLod::leaf_host(lod_ref)
	}
}
