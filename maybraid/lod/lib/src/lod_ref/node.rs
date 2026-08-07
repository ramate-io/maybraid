//! [`LodNode`] drivers that produce ephemeral [`super::LodRef`]s.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use super::LodRef;

/// Marker: this entity is an LOD driver (camera, probe, cascade track, …).
///
/// Pose history lives on [`LodNodePose`]. Filter with `F` in fine-phase systems
/// to select which nodes contribute [`LodRef`]s.
#[derive(Debug, Clone, Copy, Default, Component)]
#[require(LodNodePose)]
pub struct LodNode;

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

/// Owned pose snapshot for building [`LodRef`]s inside a system.
#[derive(Debug, Clone, Copy)]
pub struct LodNodeSnapshot {
	pub entity: Entity,
	pub previous: Transform,
	pub current: Transform,
}

/// Advance [`LodNodePose`] from each node's [`Transform`].
pub fn track_lod_nodes(mut nodes: Query<(&Transform, &mut LodNodePose), With<LodNode>>) {
	for (transform, mut pose) in &mut nodes {
		pose.previous = pose.current;
		pose.current = *transform;
	}
}

/// Collect node poses for `F`-filtered [`LodNode`]s.
pub fn collect_node_snapshots<F: bevy::ecs::query::QueryFilter>(
	nodes: &Query<(Entity, &LodNodePose), (With<LodNode>, F)>,
) -> Vec<LodNodeSnapshot> {
	nodes
		.iter()
		.map(|(entity, pose)| LodNodeSnapshot {
			entity,
			previous: pose.previous,
			current: pose.current,
		})
		.collect()
}

/// Build host-bounds [`LodRef`]s from node snapshots (borrows snapshot transforms).
pub fn lod_refs_for_bounds<'a>(
	snapshots: &'a [LodNodeSnapshot],
	bounds: &'a Aabb3d,
) -> Vec<LodRef<'a>> {
	snapshots
		.iter()
		.map(|snap| LodRef {
			entity: snap.entity,
			previous_transform: &snap.previous,
			current_transform: &snap.current,
			bounds,
		})
		.collect()
}
