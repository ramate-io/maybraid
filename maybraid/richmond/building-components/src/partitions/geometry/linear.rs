//! Full-height linear partition geometry and LOD policy.

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::LodSceneLevel;
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use crate::assets::AssetPath;
use crate::partitions::mesh_set::{mesh_child, PartitionMeshSet, PartitionMeshTier};
use crate::partitions::probe::{
	band_for_aabb, characteristic_extent, placement_center, PartitionLodBand, PartitionLodProbe,
};
use crate::placed::Placement;
use lod::lod_ref::LodRef;

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
		let center = placement_center(placement);
		let extent = characteristic_extent(placement);
		let factor = viewer.translation.distance(center) / extent;
		Self::band_from_distance_factor(factor).to_lod_scene_level()
	}

	pub fn posed_tier(
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

	pub fn posed_host(
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

	pub fn leaf_host(meshes: PartitionMeshSet, lod_ref: &LodRef) -> impl Scene + 'static {
		let band = band_for_aabb(lod_ref.bounds, lod_ref.current_transform);
		let level = band.to_lod_scene_level();
		let center = bevy_math::Vec3::from((lod_ref.bounds.min + lod_ref.bounds.max) * 0.5);
		let size = lod_ref.bounds.max - lod_ref.bounds.min;
		let probe = PartitionLodProbe {
			center,
			extent: size.x.max(size.y).max(size.z).max(1e-4),
		};
		Self::posed_host(meshes, Transform::IDENTITY, level, probe)
	}
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
