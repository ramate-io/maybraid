//! Full-height linear partition geometry and LOD policy.

use bevy::prelude::Transform;
use bevy::scene::prelude::Scene;
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;

use crate::partitions::host::{posed_mesh_tier, warm_mesh_host};
use crate::partitions::mesh_set::PartitionMeshSet;
use crate::partitions::probe::{PartitionLodBand, PartitionLodProbe};
use crate::placed::Placement;

/// `distance / max_extent` out to this → High.
pub const LINEAR_HIGH_FACTOR: f32 = 5.0;
/// Out to this → Medium.
pub const LINEAR_MEDIUM_FACTOR: f32 = 20.0;
/// Out to this → Low; else UltraLow.
pub const LINEAR_LOW_FACTOR: f32 = 500.0;

/// Default linear thickness scale (\(0.15\) world / \(0.2\) kit half-extent).
pub const DEFAULT_THICK: f32 = 0.15 / 0.2;

/// Unit linear partition (\(X \in [-1, 1]\), \(Y \in [0, 1]\), \(Z \in [-0.2, 0.2]\)).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LinearPartition;

/// LOD banding / posed mesh helpers for linear (and polyline-parent) partitions.
pub struct LinearLod;

impl LinearLod {
	pub fn band_from_distance_factor(factor: f32) -> PartitionLodBand {
		if factor <= LINEAR_HIGH_FACTOR {
			PartitionLodBand::High
		} else if factor <= LINEAR_MEDIUM_FACTOR {
			PartitionLodBand::Medium
		} else if factor <= LINEAR_LOW_FACTOR {
			PartitionLodBand::Low
		} else {
			PartitionLodBand::UltraLow
		}
	}

	pub fn level_for_placement(placement: &Placement, viewer: &Transform) -> LodSceneLevel {
		PartitionLodProbe::from_placement(placement).level_for(viewer)
	}

	pub fn posed_tier(
		meshes: PartitionMeshSet,
		transform: Transform,
		level: LodSceneLevel,
	) -> impl Scene + 'static {
		posed_mesh_tier(meshes, transform, level)
	}

	pub fn posed_host(
		meshes: PartitionMeshSet,
		transform: Transform,
		level: LodSceneLevel,
		probe: PartitionLodProbe,
	) -> impl Scene + 'static {
		warm_mesh_host(meshes, transform, level, probe)
	}

	pub fn leaf_host(meshes: PartitionMeshSet, lod_ref: &LodRef) -> impl Scene + 'static {
		let probe = PartitionLodProbe::from_aabb(lod_ref.bounds);
		let level = probe.level_for(lod_ref.current_transform);
		Self::posed_host(meshes, Transform::IDENTITY, level, probe)
	}
}
