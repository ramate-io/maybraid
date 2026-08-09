//! Cancel stale pending roots; sticky-resume desired jobs mid-teardown.

use std::time::Instant;

use bevy::prelude::*;

use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use crate::scene::level::LodSceneLevel;

use super::super::cull::{enqueue_lod_cull, LodCullEntity};
use super::types::{LodLevelRootPending, LodWantsCull};
use super::util::ms;

/// Enqueue cull for pending roots whose level is no longer desired; sticky-resume
/// desired pending roots that have not started teardown (keeps frozen plan).
pub fn cancel_stale_chunk_fulfillments(
	mut commands: Commands,
	mut cull_writer: MessageWriter<LodCullEntity>,
	hosts: Query<(Entity, &LodSceneLevel, Option<&Children>), With<LodSceneHost>>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	pending_roots: Query<&LodLevelRoot, With<LodLevelRootPending>>,
	wants_cull: Query<&LodWantsCull>,
	wants_cull_marker: Query<(), With<LodWantsCull>>,
) {
	let t0 = Instant::now();
	let mut enqueued = 0u32;
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
			if root.0 == *desired {
				if let Ok(cull) = wants_cull.get(child) {
					if !cull.started {
						commands.entity(child).remove::<LodWantsCull>();
						resumed += 1;
					}
				}
				continue;
			}
			if wants_cull_marker.contains(child) {
				continue;
			}
			enqueue_lod_cull(&mut commands, &mut cull_writer, child, &wants_cull_marker);
			enqueued += 1;
		}
	}
	if enqueued > 0 || resumed > 0 {
		info!(
			"[lod.chunk] cancel_stale: enqueued={enqueued} sticky_resumed={resumed} in {:.2}ms",
			ms(t0)
		);
	}
}
