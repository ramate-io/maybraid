//! Cancel unstarted LOD cull on pending roots that are desired again.
//!
//! Soft-paused not-desired fulfill jobs need no system here: drain skips them
//! while `root != desired` and continues automatically when desired matches.
//! This path only clears [`LodCullInFlight`] so a pending root re-enters drain
//! before teardown [`LodCullInFlight::started`].

use bevy::prelude::*;

use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use crate::scene::level::LodSceneLevel;

use super::types::{LodCullInFlight, LodLevelRootPending};

/// If a pending root is desired again and cull teardown has not started, clear
/// [`LodCullInFlight`] so spawn drain may continue the frozen fulfill queue.
pub fn cancel_unstarted_cull_for_desired_pending_roots(
	mut commands: Commands,
	cull_inflight: Query<(Entity, &LodCullInFlight, &LodLevelRoot), With<LodLevelRootPending>>,
	child_of: Query<&ChildOf>,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
) {
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
		commands.entity(entity).remove::<LodCullInFlight>();
	}
}
