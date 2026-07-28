//! Partition mesh resolution LOD (distance / extent banding).

use bevy::prelude::{Children, Component, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::{LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

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

	/// Map to shared [`LodSceneLevel`] (UltraLow collapses to Low for root identity).
	pub fn to_lod_scene_level(self) -> LodSceneLevel {
		match self.mesh_tier() {
			PartitionMeshTier::High => LodSceneLevel::High,
			PartitionMeshTier::Mid => LodSceneLevel::Medium,
			PartitionMeshTier::Low => LodSceneLevel::Low,
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
	band_for_placement(placement, lod_ref.current_transform).to_lod_scene_level()
}

fn mesh_child(asset: AssetPath) -> Box<dyn Scene> {
	Box::new(asset.mesh_ref().scene())
}

fn tier_level_root(level: LodSceneLevel, asset: AssetPath, visible: bool) -> Box<dyn Scene> {
	let children = vec![mesh_child(asset)];
	let visibility = if visible {
		Visibility::Inherited
	} else {
		Visibility::Hidden
	};
	Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		template_value(visibility)
		Children [ {children} ]
	})
}

/// Host with warm high/mid/low MeshRef level roots; active tier from `level`.
pub fn posed_partition_mesh_lod(
	meshes: PartitionMeshSet,
	transform: Transform,
	level: LodSceneLevel,
	probe: PartitionLodProbe,
) -> impl Scene + 'static {
	let roots = vec![
		tier_level_root(LodSceneLevel::High, meshes.high, level == LodSceneLevel::High),
		tier_level_root(
			LodSceneLevel::Medium,
			meshes.mid,
			level == LodSceneLevel::Medium,
		),
		tier_level_root(LodSceneLevel::Low, meshes.low, level == LodSceneLevel::Low),
	];
	let level_roots: Box<dyn Scene> = Box::new(bsn! {
		LodLevelRoots
		Transform::default()
		Visibility::Inherited
		Children [ {roots} ]
	});
	let host_children = vec![level_roots];
	bsn! {
		LodSceneHost
		template_value(level)
		template_value(probe)
		template_value(transform)
		Visibility::Inherited
		Children [ {host_children} ]
	}
}

/// Single-tier content (for `scene_with_level` / lazy spawn).
pub fn posed_partition_mesh_tier(
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
	let children = vec![mesh_child(meshes.for_tier(tier))];
	bsn! {
		template_value(transform)
		Visibility::Inherited
		Children [ {children} ]
	}
}

/// Identity-placement LOD host for playground leaf kit types.
pub fn leaf_partition_mesh_lod(
	meshes: PartitionMeshSet,
	lod_ref: &LodRef,
) -> impl Scene + 'static {
	let band = band_for_aabb(lod_ref.bounds, lod_ref.current_transform);
	let level = band.to_lod_scene_level();
	let center = Vec3::from((lod_ref.bounds.min + lod_ref.bounds.max) * 0.5);
	let size = lod_ref.bounds.max - lod_ref.bounds.min;
	let probe = PartitionLodProbe {
		center,
		extent: size.x.max(size.y).max(size.z).max(1e-4),
	};
	posed_partition_mesh_lod(meshes, Transform::IDENTITY, level, probe)
}

pub fn leaf_partition_lod_status(lod_ref: &LodRef) -> LodSceneStatus {
	let prev = band_for_aabb(lod_ref.bounds, lod_ref.previous_transform);
	let curr = band_for_aabb(lod_ref.bounds, lod_ref.current_transform);
	lod_status_for_bands(prev, curr)
}

pub fn leaf_partition_lod_level(lod_ref: &LodRef) -> LodSceneLevel {
	band_for_aabb(lod_ref.bounds, lod_ref.current_transform).to_lod_scene_level()
}

/// Fine-phase: update partition host levels from camera pose.
pub fn update_partition_host_levels(
	viewer: bevy::prelude::Query<&Transform, bevy::prelude::With<bevy::prelude::Camera3d>>,
	mut hosts: bevy::prelude::Query<
		(&PartitionLodProbe, &mut LodSceneLevel),
		bevy::prelude::With<LodSceneHost>,
	>,
) {
	let Ok(viewer_tf) = viewer.single() else {
		return;
	};
	for (probe, mut level) in &mut hosts {
		let desired = probe.level_for(viewer_tf);
		if *level != desired {
			*level = desired;
		}
	}
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
		assert!(matches!(
			lod_status_for_bands(PartitionLodBand::Low, PartitionLodBand::Medium),
			LodSceneStatus::Changed(LodSceneLevel::Medium)
		));
		Ok(())
	}
}
