//! Partition mesh-resolution policy: GLB sets → posed content for one level.
//!
//! **Split of concerns**
//! - [`crate::lod_host`] — posed level-root content (any domain).
//! - **This module** — partition **resolution policy**: which high / mid / low GLB is
//!   selected for a [`LodSceneLevel`] (and eventually a dedicated **ultra-low** asset).
//!   Until ultra-low GLBs exist, [`LodSceneLevel::UltraLow`] shares the low mesh.
//!
//! [`PartitionNode`](crate::partitions::PartitionNode) covers both **direct** kit mappings
//! (e.g. a lone linear) and **tessellated** forms (polyline / arc → many tiles under one host).

use bevy::prelude::Transform;
use bevy::scene::prelude::Scene;
use lod::gen::LodSceneLevel;
use scene_ref::MirrorAxis;

use crate::lod_host::posed_scene_ref_tier;
use crate::partitions::mesh_set::{PartitionMeshSet, PartitionMeshTier};

pub use crate::lod_host::posed_asset_tier;

/// Resolution tier for a level. UltraLow uses the low GLB until a fourth path is authored.
pub fn mesh_tier_for_level(level: LodSceneLevel) -> PartitionMeshTier {
	match level {
		LodSceneLevel::High => PartitionMeshTier::High,
		LodSceneLevel::Medium => PartitionMeshTier::Mid,
		LodSceneLevel::Low | LodSceneLevel::UltraLow => PartitionMeshTier::Low,
		LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => PartitionMeshTier::Mid,
	}
}

/// One SceneRef under a transform (for `scene_with_level`).
pub fn posed_mesh_tier(
	meshes: PartitionMeshSet,
	transform: Transform,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	posed_mirrored_mesh_tier(meshes, transform, level, None)
}

/// Like [`posed_mesh_tier`], with optional [`scene_ref::SceneRef`] axis mirroring.
pub fn posed_mirrored_mesh_tier(
	meshes: PartitionMeshSet,
	transform: Transform,
	level: LodSceneLevel,
	mirror: Option<MirrorAxis>,
) -> impl Scene + 'static {
	let asset = meshes.for_tier(mesh_tier_for_level(level));
	posed_scene_ref_tier(Some(asset.scene_ref().with_mirror(mirror)), transform)
}
