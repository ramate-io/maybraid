//! Partition mesh resolution LOD (distance / extent banding).

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneStatus;
use lod::lod_ref::LodRef;

use crate::assets::AssetPath;
use crate::placed::Placement;

/// Viewer distance band for partition mesh resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionLodBand {
	/// Farthest; shares [`PartitionMeshTier::Low`] until a shared ultra-low exists.
	UltraLow,
	Low,
	Medium,
	High,
}

/// Which of the three warm MeshRef children is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionMeshTier {
	Low,
	Mid,
	High,
}

/// `distance / characteristic_extent` below this → [`PartitionLodBand::High`].
pub const PARTITION_HIGH_FACTOR: f32 = 2.0;
/// Below this → [`PartitionLodBand::Medium`].
pub const PARTITION_MEDIUM_FACTOR: f32 = 5.0;
/// Below this → [`PartitionLodBand::Low`]; else [`PartitionLodBand::UltraLow`].
pub const PARTITION_LOW_FACTOR: f32 = 12.0;

impl PartitionLodBand {
	pub fn from_distance_factor(factor: f32) -> Self {
		if factor < PARTITION_HIGH_FACTOR {
			Self::High
		} else if factor < PARTITION_MEDIUM_FACTOR {
			Self::Medium
		} else if factor < PARTITION_LOW_FACTOR {
			Self::Low
		} else {
			Self::UltraLow
		}
	}

	/// UltraLow and Low share the low-res mesh until a dedicated ultra-low exists.
	pub fn mesh_tier(self) -> PartitionMeshTier {
		match self {
			Self::UltraLow | Self::Low => PartitionMeshTier::Low,
			Self::Medium => PartitionMeshTier::Mid,
			Self::High => PartitionMeshTier::High,
		}
	}
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

	/// All tiers use the same asset (e.g. kits without resolution variants).
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

/// Characteristic size from placement scale (max axis).
pub fn characteristic_extent(placement: &Placement) -> f32 {
	placement
		.scale
		.x
		.max(placement.scale.y)
		.max(placement.scale.z)
		.max(1e-4)
}

/// Approximate world center of a placed partition (mid-height of kit).
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
	// Mesh tier collapses UltraLow≡Low; only tier changes force a scene rebuild.
	if prev.mesh_tier() == curr.mesh_tier() {
		LodSceneStatus::Unchanged
	} else {
		LodSceneStatus::Changed
	}
}

pub fn lod_status_for_placement(placement: &Placement, lod_ref: &LodRef) -> LodSceneStatus {
	let prev = band_for_placement(placement, lod_ref.previous_transform);
	let curr = band_for_placement(placement, lod_ref.current_transform);
	lod_status_for_bands(prev, curr)
}

/// Spawn high/mid/low MeshRefs under `transform`; hide inactive tiers so assets stay warm.
pub fn posed_partition_mesh_lod(
	meshes: PartitionMeshSet,
	transform: Transform,
	tier: PartitionMeshTier,
) -> impl Scene + 'static {
	let high_child: Box<dyn Scene> = Box::new((
		meshes.high.mesh_ref().scene(),
		bsn! {
			Transform::default()
			template_value(tier_visibility(tier, PartitionMeshTier::High))
		},
	));
	let mid_child: Box<dyn Scene> = Box::new((
		meshes.mid.mesh_ref().scene(),
		bsn! {
			Transform::default()
			template_value(tier_visibility(tier, PartitionMeshTier::Mid))
		},
	));
	let low_child: Box<dyn Scene> = Box::new((
		meshes.low.mesh_ref().scene(),
		bsn! {
			Transform::default()
			template_value(tier_visibility(tier, PartitionMeshTier::Low))
		},
	));
	let children = vec![high_child, mid_child, low_child];
	bsn! {
		template_value(transform)
		Visibility::Inherited
		Children [ {children} ]
	}
}

fn tier_visibility(active: PartitionMeshTier, slot: PartitionMeshTier) -> Visibility {
	if active == slot {
		Visibility::Inherited
	} else {
		Visibility::Hidden
	}
}

/// Identity-placement LOD scene for playground leaf kit types.
pub fn leaf_partition_mesh_lod(
	meshes: PartitionMeshSet,
	lod_ref: &LodRef,
) -> impl Scene + 'static {
	let band = band_for_aabb(lod_ref.bounds, lod_ref.current_transform);
	posed_partition_mesh_lod(meshes, Transform::IDENTITY, band.mesh_tier())
}

pub fn leaf_partition_lod_status(lod_ref: &LodRef) -> LodSceneStatus {
	let prev = band_for_aabb(lod_ref.bounds, lod_ref.previous_transform);
	let curr = band_for_aabb(lod_ref.bounds, lod_ref.current_transform);
	lod_status_for_bands(prev, curr)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn distance_factor_maps_to_bands() -> anyhow::Result<()> {
		assert_eq!(PartitionLodBand::from_distance_factor(1.0), PartitionLodBand::High);
		assert_eq!(PartitionLodBand::from_distance_factor(3.0), PartitionLodBand::Medium);
		assert_eq!(PartitionLodBand::from_distance_factor(8.0), PartitionLodBand::Low);
		assert_eq!(
			PartitionLodBand::from_distance_factor(20.0),
			PartitionLodBand::UltraLow
		);
		assert_eq!(
			PartitionLodBand::UltraLow.mesh_tier(),
			PartitionMeshTier::Low
		);
		assert_eq!(PartitionLodBand::Low.mesh_tier(), PartitionMeshTier::Low);
		Ok(())
	}

	#[test]
	fn ultralow_and_low_share_tier_status() -> anyhow::Result<()> {
		assert_eq!(
			lod_status_for_bands(PartitionLodBand::UltraLow, PartitionLodBand::Low),
			LodSceneStatus::Unchanged
		);
		assert_eq!(
			lod_status_for_bands(PartitionLodBand::Low, PartitionLodBand::Medium),
			LodSceneStatus::Changed
		);
		Ok(())
	}
}
