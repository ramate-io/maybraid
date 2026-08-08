//! [`LodNode`] drivers that produce ephemeral [`super::LodRef`]s.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use super::LodRef;

/// Marker: this entity is an LOD driver (camera, probe, cascade track, …).
///
/// Pose history lives on [`LodNodePose`]. Filter with `F` in refresh systems
/// to select which nodes contribute [`LodRef`]s.
#[derive(Debug, Clone, Copy, Default, Component)]
#[require(LodNodePose)]
pub struct LodNode;

/// Driver extents for [`LodRef::bounds`].
///
/// If absent, the node is treated as pointlike at [`LodNodePose::current`] translation.
#[derive(Debug, Clone, Copy, Component)]
pub struct LodNodeBounds(pub Aabb3d);

/// Previous / current transform for a [`LodNode`] (for ephemeral [`LodRef`]s).
#[derive(Debug, Clone, Copy, Component)]
pub struct LodNodePose {
	pub previous: Transform,
	pub current: Transform,
}

impl Default for LodNodePose {
	fn default() -> Self {
		Self {
			previous: Transform::IDENTITY,
			current: Transform::IDENTITY,
		}
	}
}

impl LodNodePose {
	/// Borrowed [`LodRef`] for this pose + driver `bounds`.
	pub fn as_lod_ref<'a>(&'a self, entity: Entity, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity,
			previous_transform: &self.previous,
			current_transform: &self.current,
			bounds,
		}
	}
}

/// Owned pose + bounds snapshot for building [`LodRef`]s inside a system.
#[derive(Debug, Clone, Copy)]
pub struct LodNodeSnapshot {
	pub entity: Entity,
	pub previous: Transform,
	pub current: Transform,
	/// Driver extents (from [`LodNodeBounds`], or a point at `current.translation`).
	pub bounds: Aabb3d,
}

/// Advance [`LodNodePose`] from each node's [`Transform`].
///
/// Runs every frame (not [`Changed<Transform>`]): `previous`/`current` are a
/// one-frame sliding window. Filtering to changed transforms would leave
/// `previous != current` after motion stops, so region strategies that key off
/// that delta would keep firing.
pub fn track_lod_nodes(mut nodes: Query<(&Transform, &mut LodNodePose), With<LodNode>>) {
	for (transform, mut pose) in &mut nodes {
		pose.previous = pose.current;
		pose.current = *transform;
	}
}

/// Pointlike driver AABB at `translation`.
pub fn point_bounds(translation: Vec3) -> Aabb3d {
	Aabb3d::from_min_max(translation, translation)
}

/// Collect node poses/bounds for `F`-filtered [`LodNode`]s.
pub fn collect_node_snapshots<F: bevy::ecs::query::QueryFilter>(
	nodes: &Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
) -> Vec<LodNodeSnapshot> {
	nodes
		.iter()
		.map(|(entity, pose, bounds)| LodNodeSnapshot {
			entity,
			previous: pose.previous,
			current: pose.current,
			bounds: bounds
				.map(|b| b.0)
				.unwrap_or_else(|| point_bounds(pose.current.translation)),
		})
		.collect()
}

/// Build one [`LodRef`] per snapshot (driver pose + that driver's bounds).
pub fn lod_refs_from_snapshots<'a>(snapshots: &'a [LodNodeSnapshot]) -> Vec<LodRef<'a>> {
	snapshots
		.iter()
		.map(|snap| LodRef {
			entity: snap.entity,
			previous_transform: &snap.previous,
			current_transform: &snap.current,
			bounds: &snap.bounds,
		})
		.collect()
}
