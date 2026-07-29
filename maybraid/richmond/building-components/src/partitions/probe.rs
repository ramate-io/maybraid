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

	pub fn status_vs(self, prev: Self) -> LodSceneStatus {
		let prev_l = prev.to_lod_scene_level();
		let curr_l = self.to_lod_scene_level();
		if prev_l == curr_l {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr_l)
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
			center: Self::placement_center(placement),
			extent: Self::characteristic_extent(placement),
		}
	}

	pub fn from_aabb(aabb: &Aabb3d) -> Self {
		let center = Vec3::from((aabb.min + aabb.max) * 0.5);
		let size = aabb.max - aabb.min;
		Self {
			center,
			extent: size.x.max(size.y).max(size.z).max(1e-4),
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

	pub fn band_for(&self, viewer: &Transform) -> PartitionLodBand {
		let factor = viewer.translation.distance(self.center) / self.extent.max(1e-4);
		PartitionLodBand::from_distance_factor(factor)
	}

	pub fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		self.band_for(viewer).to_lod_scene_level()
	}

	pub fn status_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.band_for(lod_ref.current_transform)
			.status_vs(self.band_for(lod_ref.previous_transform))
	}
}

impl Placement {
	pub fn partition_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		PartitionLodProbe::from_placement(self).level_for(lod_ref.current_transform)
	}

	pub fn partition_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		PartitionLodProbe::from_placement(self).status_for_lod_ref(lod_ref)
	}
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
