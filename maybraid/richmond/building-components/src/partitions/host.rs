//! Partition mesh-resolution hosts: GLB sets → crate [`lod_host`](crate::lod_host) scaffolding.
//!
//! **Split of concerns**
//! - [`crate::lod_host`] — structural warm `LodSceneHost` / level roots (any domain).
//! - **This module** — partition **resolution policy**: which high / mid / low GLBs
//!   sit in those roots (and eventually a dedicated **ultra-low** asset). Until ultra-low
//!   GLBs exist, [`LodSceneLevel::UltraLow`] shares the low mesh via banding.
//!
//! [`PartitionNode`](crate::partitions::PartitionNode) covers both **direct** kit mappings
//! (e.g. a lone linear) and **tessellated** forms (polyline / arc → many tiles under one parent host).

use bevy::prelude::Transform;
use bevy::scene::prelude::Scene;
use lod::gen::LodSceneLevel;

use crate::assets::AssetPath;
use crate::lod_host::warm_mesh_level_host;
use crate::partitions::mesh_set::{PartitionMeshSet, PartitionMeshTier};
use crate::partitions::probe::PartitionLodProbe;

pub use crate::lod_host::posed_asset_tier;

/// One MeshRef under a transform (for `scene_with_level`).
///
/// UltraLow uses the low GLB until a fourth ultra-low path is authored.
pub fn posed_mesh_tier(
	meshes: PartitionMeshSet,
	transform: Transform,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let tier = match level {
		LodSceneLevel::High => PartitionMeshTier::High,
		LodSceneLevel::Medium => PartitionMeshTier::Mid,
		LodSceneLevel::Low | LodSceneLevel::UltraLow => PartitionMeshTier::Low,
		LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => PartitionMeshTier::Mid,
	};
	posed_asset_tier(Some(meshes.for_tier(tier)), transform)
}

/// Warm high / mid / low mesh roots under one host.
///
/// When ultra-low GLBs ship, extend [`PartitionMeshSet`] and pass a fourth
/// `(LodSceneLevel::UltraLow, Some(meshes.ultra_low))` into [`warm_mesh_level_host`].
pub fn warm_mesh_host(
	meshes: PartitionMeshSet,
	transform: Transform,
	level: LodSceneLevel,
	probe: PartitionLodProbe,
) -> impl Scene + 'static {
	warm_mesh_level_host(
		level,
		probe,
		transform,
		[
			(LodSceneLevel::High, Some(meshes.high)),
			(LodSceneLevel::Medium, Some(meshes.mid)),
			(LodSceneLevel::Low, Some(meshes.low)),
		],
	)
}

/// Warm host with optional per-level mesh (e.g. joint high/mid, empty low).
pub fn warm_host(
	level: LodSceneLevel,
	probe: PartitionLodProbe,
	transform: Transform,
	roots: [(LodSceneLevel, Option<AssetPath>); 3],
) -> impl Scene + 'static {
	warm_mesh_level_host(level, probe, transform, roots)
}
