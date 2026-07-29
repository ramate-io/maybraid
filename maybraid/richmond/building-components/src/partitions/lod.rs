//! Partition LOD re-exports and parent-host builders.
//!
//! Banding policy lives on geometry modules ([`LinearLod`](crate::partitions::geometry::LinearLod),
//! [`JointLod`](crate::partitions::geometry::JointLod)). This module keeps shared probes and
//! the single-parent warm host used by [`PartitionNode`](crate::partitions::PartitionNode).

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use crate::partitions::geometry::LinearLod;
use crate::partitions::mesh_set::PartitionMeshSet as MeshSet;

pub use crate::partitions::geometry::{
	LINEAR_HIGH_FACTOR as PARTITION_HIGH_FACTOR, LINEAR_LOW_FACTOR as PARTITION_LOW_FACTOR,
	LINEAR_MEDIUM_FACTOR as PARTITION_MEDIUM_FACTOR,
};
pub use crate::partitions::mesh_set::{PartitionMeshSet, PartitionMeshTier};
pub use crate::partitions::probe::{
	band_for_aabb, band_for_placement, characteristic_extent, leaf_partition_lod_level,
	leaf_partition_lod_status, lod_level_for_placement, lod_status_for_bands,
	lod_status_for_placement, placement_center, update_partition_host_levels, PartitionLodBand,
	PartitionLodProbe,
};

/// Identity-placement LOD host for playground leaf kit types (linear-style banding).
pub fn leaf_partition_mesh_lod(meshes: MeshSet, lod_ref: &LodRef) -> impl Scene + 'static {
	LinearLod::leaf_host(meshes, lod_ref)
}

pub fn posed_partition_mesh_tier(
	meshes: MeshSet,
	transform: Transform,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	LinearLod::posed_tier(meshes, transform, level)
}

pub fn posed_partition_mesh_lod(
	meshes: MeshSet,
	transform: Transform,
	level: LodSceneLevel,
	probe: PartitionLodProbe,
) -> impl Scene + 'static {
	LinearLod::posed_host(meshes, transform, level, probe)
}

/// Single parent host with three warm level roots; each root holds `content(level)`.
pub fn posed_partition_parent_lod(
	level: LodSceneLevel,
	probe: PartitionLodProbe,
	high: impl Scene + 'static,
	mid: impl Scene + 'static,
	low: impl Scene + 'static,
) -> impl Scene + 'static {
	let roots = vec![
		content_level_root(LodSceneLevel::High, high, level == LodSceneLevel::High),
		content_level_root(LodSceneLevel::Medium, mid, level == LodSceneLevel::Medium),
		content_level_root(LodSceneLevel::Low, low, level == LodSceneLevel::Low),
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
		Transform::default()
		Visibility::Inherited
		Children [ {host_children} ]
	}
}

fn content_level_root(
	level: LodSceneLevel,
	content: impl Scene + 'static,
	visible: bool,
) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = vec![Box::new(content)];
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
