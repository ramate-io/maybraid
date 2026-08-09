//! Write desired [`LodSceneLevel`] on hosts from [`LodNode`] drivers.

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose, LodRef,
};
use crate::scene::host::{nested_host_parent_allows_refresh, LodSceneHost};
use crate::scene::level::LodSceneLevel;
use crate::scene::LodScene;

/// Pick the driver ref that votes for the highest [`LodSceneLevel`].
pub fn dominant_lod_ref<'a, T: LodScene>(
	scene: &T,
	refs: &'a [LodRef<'a>],
) -> Option<&'a LodRef<'a>> {
	refs.iter().max_by_key(|lod_ref| scene.scene_lod_level(lod_ref))
}

/// Set host [`LodSceneLevel`] from `FNode`-filtered [`LodNode`]s.
///
/// Probe / unscoped path (no region messages). `FHost` scopes hosts (`()` = all).
/// [`LodRef`] bounds come from each node's [`LodNodeBounds`] (or a point).
/// Nested hosts under a non-[`LodSceneLevel::High`] parent host are skipped.
pub fn update_lod_host_levels<T, FHost, FNode>(
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, FNode)>,
	child_of: Query<&ChildOf>,
	mut sets: ParamSet<(
		(
			Query<&'static LodSceneLevel, With<LodSceneHost>>,
			Query<(Entity, &'static T), (With<LodSceneHost>, FHost)>,
		),
		Query<(Entity, &'static mut LodSceneLevel), (With<LodSceneHost>, With<T>, FHost)>,
	)>,
) where
	T: Component + LodScene,
	FHost: QueryFilter + 'static,
	FNode: QueryFilter + 'static,
{
	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);
	let ref_refs: Vec<_> = refs.iter().collect();
	let t0 = std::time::Instant::now();

	let desired: Vec<(Entity, LodSceneLevel)> = {
		let (host_levels, hosts) = sets.p0();
		hosts
			.iter()
			.filter(|(entity, _)| {
				nested_host_parent_allows_refresh(*entity, &child_of, &host_levels)
			})
			.map(|(entity, scene)| (entity, scene.scene_lod_level_from_levels(&ref_refs)))
			.collect()
	};

	let mut changed = 0u32;
	let n = desired.len() as u32;
	{
		let mut levels = sets.p1();
		for (entity, want) in desired {
			let Ok((_, mut level)) = levels.get_mut(entity) else {
				continue;
			};
			if *level != want {
				*level = want;
				changed += 1;
			}
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
