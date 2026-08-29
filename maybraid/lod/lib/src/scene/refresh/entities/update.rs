//! Write desired [`LodSceneLevel`] on hosts from [`LodNode`] drivers.

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose, LodRef,
};
use crate::scene::host::{
	nested_host_parent_allows_refresh, LodLevelRoot, LodLevelRoots, LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::visual::{under_visual_lod_root, VisualLodRoot};
use crate::scene::SemanticLodScene;

/// Pick the driver ref that votes for the highest [`LodSceneLevel`].
pub fn dominant_lod_ref<'a, T: SemanticLodScene>(
	scene: &T,
	refs: &'a [LodRef<'a>],
) -> Option<&'a LodRef<'a>> {
	refs.iter().max_by_key(|lod_ref| scene.scene_lod_level(lod_ref))
}

/// Set host [`LodSceneLevel`] from `FNode`-filtered [`LodNode`]s.
///
/// Probe / unscoped path (no region messages). `FHost` scopes hosts (`()` = all).
/// [`LodRef`] bounds come from each node's [`LodNodeBounds`] (or a point).
/// Nested hosts not under their parent's desired or shown [`LodLevelRoot`] are skipped.
pub fn update_lod_host_levels<T, FHost, FNode>(
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, FNode)>,
	child_of: Query<&ChildOf>,
	level_roots: Query<&LodLevelRoot>,
	children_q: Query<&Children>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	visibilities: Query<&Visibility>,
	mut sets: ParamSet<(
		(
			Query<&'static LodSceneLevel, With<LodSceneHost>>,
			Query<(Entity, &'static T), (With<LodSceneHost>, FHost)>,
		),
		Query<(Entity, &'static mut LodSceneLevel), (With<LodSceneHost>, With<T>, FHost)>,
	)>,
	visual_roots: Query<(), With<VisualLodRoot>>,
) where
	T: Component + SemanticLodScene,
	FHost: QueryFilter + 'static,
	FNode: QueryFilter + 'static,
{
	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);

	let desired: Vec<(Entity, LodSceneLevel)> = {
		let (host_levels, hosts) = sets.p0();
		hosts
			.iter()
			.filter(|(entity, _)| {
				if under_visual_lod_root(*entity, &child_of, &visual_roots) {
					return false;
				}
				nested_host_parent_allows_refresh(
					*entity,
					&child_of,
					&host_levels,
					&level_roots,
					&children_q,
					&level_roots_bags,
					&visibilities,
				)
			})
			.map(|(entity, scene)| (entity, scene.scene_lod_level_from_levels(&refs)))
			.collect()
	};

	{
		let mut levels = sets.p1();
		for (entity, want) in desired {
			let Ok((_, mut level)) = levels.get_mut(entity) else {
				continue;
			};
			if *level != want {
				*level = want;
			}
		}
	}
}
