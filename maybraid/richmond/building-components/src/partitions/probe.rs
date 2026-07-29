//! Distance / extent probe shared by partition LOD hosts.

use bevy::prelude::{Component, Query, Res, Transform, With};
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::{LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;

use crate::partitions::geometry::LinearLod;
use crate::placed::Placement;

/// Viewer distance band for partition mesh resolution (linear / polyline parent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionLodBand {
	UltraLow,
	Low,
	Medium,
	High,
}

impl PartitionLodBand {
	pub fn from_distance_factor(factor: f32) -> Self {
		LinearLod::band_from_distance_factor(factor)
	}

	pub fn mesh_tier(self) -> crate::partitions::mesh_set::PartitionMeshTier {
		use crate::partitions::mesh_set::PartitionMeshTier;
		match self {
			Self::UltraLow | Self::Low => PartitionMeshTier::Low,
			Self::Medium => PartitionMeshTier::Mid,
			Self::High => PartitionMeshTier::High,
		}
	}

	pub fn to_lod_scene_level(self) -> LodSceneLevel {
		match self.mesh_tier() {
			crate::partitions::mesh_set::PartitionMeshTier::High => LodSceneLevel::High,
			crate::partitions::mesh_set::PartitionMeshTier::Mid => LodSceneLevel::Medium,
			crate::partitions::mesh_set::PartitionMeshTier::Low => LodSceneLevel::Low,
		}
	}
}

/// Fine-phase probe for partition mesh hosts (center + characteristic extent).
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct PartitionLodProbe {
	pub center: Vec3,
	pub extent: f32,
}

impl PartitionLodProbe {
	pub fn from_placement(placement: &Placement) -> Self {
		Self {
			center: placement_center(placement),
			extent: characteristic_extent(placement),
		}
	}

	pub fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		let factor = viewer.translation.distance(self.center) / self.extent.max(1e-4);
		PartitionLodBand::from_distance_factor(factor).to_lod_scene_level()
	}
}

pub fn characteristic_extent(placement: &Placement) -> f32 {
	placement
		.scale
		.x
		.max(placement.scale.y)
		.max(placement.scale.z)
		.max(1e-4)
}

pub fn placement_center(placement: &Placement) -> Vec3 {
	placement.translation + Vec3::new(0.0, placement.scale.y * 0.5, 0.0)
}

pub fn band_for_placement(placement: &Placement, viewer: &Transform) -> PartitionLodBand {
	let center = placement_center(placement);
	let extent = characteristic_extent(placement);
	let factor = viewer.translation.distance(center) / extent;
	PartitionLodBand::from_distance_factor(factor)
}

pub fn band_for_aabb(aabb: &Aabb3d, viewer: &Transform) -> PartitionLodBand {
	let center = Vec3::from((aabb.min + aabb.max) * 0.5);
	let size = aabb.max - aabb.min;
	let extent = size.x.max(size.y).max(size.z).max(1e-4);
	let factor = viewer.translation.distance(center) / extent;
	PartitionLodBand::from_distance_factor(factor)
}

pub fn lod_status_for_bands(prev: PartitionLodBand, curr: PartitionLodBand) -> LodSceneStatus {
	let prev_l = prev.to_lod_scene_level();
	let curr_l = curr.to_lod_scene_level();
	if prev_l == curr_l {
		LodSceneStatus::Unchanged
	} else {
		LodSceneStatus::Changed(curr_l)
	}
}

pub fn lod_status_for_placement(placement: &Placement, lod_ref: &LodRef) -> LodSceneStatus {
	let prev = band_for_placement(placement, lod_ref.previous_transform);
	let curr = band_for_placement(placement, lod_ref.current_transform);
	lod_status_for_bands(prev, curr)
}

pub fn lod_level_for_placement(placement: &Placement, lod_ref: &LodRef) -> LodSceneLevel {
	LinearLod::level_for_placement(placement, lod_ref.current_transform)
}

pub fn leaf_partition_lod_status(lod_ref: &LodRef) -> LodSceneStatus {
	let prev = band_for_aabb(lod_ref.bounds, lod_ref.previous_transform);
	let curr = band_for_aabb(lod_ref.bounds, lod_ref.current_transform);
	lod_status_for_bands(prev, curr)
}

pub fn leaf_partition_lod_level(lod_ref: &LodRef) -> LodSceneLevel {
	band_for_aabb(lod_ref.bounds, lod_ref.current_transform).to_lod_scene_level()
}

/// Fine-phase: update partition host levels from [`lod::LodViewerState`].
pub fn update_partition_host_levels(
	viewer: Res<lod::LodViewerState>,
	mut hosts: Query<(&PartitionLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	if viewer.entity == bevy::prelude::Entity::PLACEHOLDER {
		return;
	}
	for (probe, mut level) in &mut hosts {
		let desired = probe.level_for(&viewer.current);
		if *level != desired {
			*level = desired;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::partitions::geometry::{LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR};

	#[test]
	fn distance_factor_maps_to_bands() -> anyhow::Result<()> {
		assert_eq!(
			PartitionLodBand::from_distance_factor(LINEAR_HIGH_FACTOR),
			PartitionLodBand::High
		);
		assert_eq!(
			PartitionLodBand::from_distance_factor(LINEAR_MEDIUM_FACTOR),
			PartitionLodBand::Medium
		);
		assert_eq!(
			PartitionLodBand::from_distance_factor(LINEAR_LOW_FACTOR),
			PartitionLodBand::Low
		);
		assert_eq!(
			PartitionLodBand::from_distance_factor(LINEAR_LOW_FACTOR + 1.0),
			PartitionLodBand::UltraLow
		);
		Ok(())
	}
}
