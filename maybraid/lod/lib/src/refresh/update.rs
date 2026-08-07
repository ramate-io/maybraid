//! Write desired [`LodSceneLevel`] on hosts from [`LodNode`] drivers.

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::gen::LodScene;
use crate::lod_level::LodSceneLevel;
use crate::lod_scene_host::LodSceneHost;

use super::bounds::LodHostBounds;
use super::node::{collect_node_snapshots, lod_refs_for_bounds, LodNode, LodNodePose};

/// Set host [`LodSceneLevel`] from `FNode`-filtered [`LodNode`]s + [`LodHostBounds`].
///
/// `FHost` scopes hosts (`()` = all; `With<LodRefresh>` for marked refresh).
pub fn update_lod_host_levels<T, FHost, FNode>(
	nodes: Query<(Entity, &LodNodePose), (With<LodNode>, FNode)>,
	mut hosts: Query<
		(&T, &LodHostBounds, &mut LodSceneLevel),
		(With<LodSceneHost>, FHost),
	>,
) where
	T: Component + LodScene,
	FHost: QueryFilter + 'static,
	FNode: QueryFilter + 'static,
{
	let snapshots = collect_node_snapshots(&nodes);
	let t0 = std::time::Instant::now();
	let mut changed = 0u32;
	let mut n = 0u32;
	for (scene, bounds, mut level) in &mut hosts {
		n += 1;
		let refs = lod_refs_for_bounds(&snapshots, &bounds.0);
		let ref_refs: Vec<_> = refs.iter().collect();
		let desired = scene.scene_lod_level_from_levels(&ref_refs);
		if *level != desired {
			*level = desired;
			changed += 1;
		}
	}
	let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
	if changed > 0 || elapsed_ms >= 0.5 {
		info!(
			"[lod.refresh] update_lod_host_levels: hosts={n} nodes={} changed={changed} in {elapsed_ms:.2}ms",
			snapshots.len()
		);
	}
}
