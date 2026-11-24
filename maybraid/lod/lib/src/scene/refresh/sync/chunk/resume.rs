//! Cancel unstarted LOD cull on pending roots that are desired again.
//!
//! Soft-paused not-desired fulfill jobs need no system here: drain skips them
//! while `root != desired` and continues automatically when desired matches.
//! This path only clears [`LodCullInFlight`] so a pending root re-enters drain
//! before teardown [`LodCullInFlight::started`].

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;

use crate::lod_ref::{
	lod_refs_from_snapshots, point_bounds, LodNode, LodNodeBounds, LodNodePose, LodNodeSnapshot,
};
use crate::scene::host::{LodLevelRoot, LodLevelRoots};
use crate::scene::level::LodSceneLevel;
use crate::scene::LodLevelProducer;

use super::super::super::viewer::LodViewer;
use super::types::{LodCullInFlight, LodLevelRootPending};

/// Type-erased resume pass shared by every semantic host type.
///
/// If a pending root is desired again and cull teardown has not started, clear
/// [`LodCullInFlight`] so spawn drain may continue the frozen fulfill queue.
///
/// Does **not** resume a root that [`crate::LodScene::scene_lod_culls`] still
/// wants gone (stale High after the camera left the bullseye).
pub fn cancel_unstarted_cull_for_desired_pending_roots_erased(world: &mut World) {
	let snapshots: Vec<LodNodeSnapshot> = {
		let mut query = world.query_filtered::<
			(Entity, &LodNodePose, Option<&LodNodeBounds>),
			(With<LodNode>, With<LodViewer>),
		>();
		query
			.iter(world)
			.map(|(entity, pose, bounds)| LodNodeSnapshot {
				entity,
				previous: pose.previous,
				current: pose.current,
				bounds: bounds
					.map(|bounds| bounds.0)
					.unwrap_or_else(|| point_bounds(pose.current.translation)),
			})
			.collect()
	};
	let refs = lod_refs_from_snapshots(&snapshots);
	let roots: Vec<_> = {
		let mut query = world.query_filtered::<
			(Entity, &LodCullInFlight, &LodLevelRoot),
			(With<LodLevelRootPending>, Allow<Disabled>),
		>();
		query
			.iter(world)
			.filter_map(|(entity, cull, root)| (!cull.started).then_some((entity, *root)))
			.collect()
	};

	for (entity, root) in roots {
		let Some(bag) = world.get::<ChildOf>(entity).map(|parent| parent.parent()) else {
			continue;
		};
		if world.get::<LodLevelRoots>(bag).is_none() {
			continue;
		}
		let Some(host) = world.get::<ChildOf>(bag).map(|parent| parent.parent()) else {
			continue;
		};
		let Some(desired) = world.get::<LodSceneLevel>(host).copied() else {
			continue;
		};
		if root.0 != desired {
			continue;
		}
		if !refs.is_empty() {
			let Some(producer) = world.get::<LodLevelProducer>(host).copied() else {
				continue;
			};
			if producer.culls_root_from_all(world, host, &refs, desired, root.0) {
				continue;
			}
		}
		world.entity_mut(entity).remove::<LodCullInFlight>();
	}
}
