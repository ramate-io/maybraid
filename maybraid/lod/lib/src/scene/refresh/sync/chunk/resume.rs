//! Resume pending roots that are desired again after a cull request was applied
//! but teardown has not started.
//!
//! Not-desired fulfill jobs are **paused** by drain/begin (desired check) — they are
//! not marked for cull here. Queue and `LodChunkFulfillment` stay intact.

use std::time::Instant;

use bevy::prelude::*;

use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use crate::scene::level::LodSceneLevel;

use super::types::{LodCullInFlight, LodLevelRootPending};
use super::util::ms;

/// If a pending root is desired again and cull teardown has not started, clear
/// [`LodCullInFlight`] so spawn drain may continue the frozen fulfill queue.
pub fn resume_desired_pending_roots(
	mut commands: Commands,
	hosts: Query<(Entity, &LodSceneLevel, Option<&Children>), With<LodSceneHost>>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	pending_roots: Query<&LodLevelRoot, With<LodLevelRootPending>>,
	cull_inflight: Query<&LodCullInFlight>,
) {
	let t0 = Instant::now();
	let mut resumed = 0u32;
	for (_host, desired, host_children) in &hosts {
		let Some(host_children) = host_children else {
			continue;
		};
		let mut roots_entity = None;
		for child in host_children.iter() {
			if level_roots_heads.contains(child) {
				roots_entity = Some(child);
				break;
			}
		}
		let Some(roots_entity) = roots_entity else {
			continue;
		};
		let Ok(root_children) = level_roots_heads.get(roots_entity) else {
			continue;
		};
		for child in root_children.iter() {
			let Ok(root) = pending_roots.get(child) else {
				continue;
			};
			if root.0 != *desired {
				continue;
			}
			let Ok(cull) = cull_inflight.get(child) else {
				continue;
			};
			if cull.started {
				continue;
			}
			commands.entity(child).remove::<LodCullInFlight>();
			resumed += 1;
		}
	}
	if resumed > 0 {
		info!(
			"[lod.chunk] resume_desired: resumed={resumed} in {:.2}ms",
			ms(t0)
		);
	}
}
