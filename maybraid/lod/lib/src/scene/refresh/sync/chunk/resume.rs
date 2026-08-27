//! Cancel unstarted LOD cull on pending roots that are desired again.
//!
//! Soft-paused not-desired fulfill jobs need no system here: drain skips them
//! while `root != desired` and continues automatically when desired matches.
//! This path only clears [`LodCullInFlight`] so a pending root re-enters drain
//! before teardown [`LodCullInFlight::started`].

use bevy::prelude::*;

use crate::lod_ref::{point_bounds, LodNode, LodNodeBounds, LodNodePose};
use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use crate::scene::level::LodSceneLevel;
use crate::scene::LodScene;

use super::super::super::viewer::LodViewer;
use super::types::{LodCullInFlight, LodLevelRootPending};

/// If a pending root is desired again and cull teardown has not started, clear
/// [`LodCullInFlight`] so spawn drain may continue the frozen fulfill queue.
///
/// Does **not** resume a root that [`LodScene::scene_lod_culls`] still wants
/// gone (stale High after the camera left the bullseye).
pub fn cancel_unstarted_cull_for_desired_pending_roots<T: Component + LodScene>(
	mut commands: Commands,
	cull_inflight: Query<(Entity, &LodCullInFlight, &LodLevelRoot), With<LodLevelRootPending>>,
	child_of: Query<&ChildOf>,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	scenes: Query<&T, With<LodSceneHost>>,
	viewer: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, With<LodViewer>)>,
) {
	let viewer = viewer.single().ok();
	let driver_bounds = viewer.as_ref().map(|(_, pose, viewer_bounds)| {
		viewer_bounds
			.map(|b| b.0)
			.unwrap_or_else(|| point_bounds(pose.current.translation))
	});

	for (entity, cull, root) in &cull_inflight {
		if cull.started {
			continue;
		}
		// root → LodLevelRoots bag → LodSceneHost
		let Ok(bag_of) = child_of.get(entity) else {
			continue;
		};
		let bag = bag_of.parent();
		if !level_roots_bags.contains(bag) {
			continue;
		}
		let Ok(host_of) = child_of.get(bag) else {
			continue;
		};
		let host = host_of.parent();
		let Ok(desired) = host_levels.get(host) else {
			continue;
		};
		if root.0 != *desired {
			continue;
		}
		let Ok(scene) = scenes.get(host) else {
			continue;
		};
		if let (Some((viewer_entity, pose, _)), Some(bounds)) =
			(viewer.as_ref(), driver_bounds.as_ref())
		{
			let lod_ref = pose.as_lod_ref(*viewer_entity, bounds);
			if scene.scene_lod_culls(&lod_ref, *desired).should_cull(root.0) {
				continue;
			}
		}
		commands.entity(entity).remove::<LodCullInFlight>();
	}
}
