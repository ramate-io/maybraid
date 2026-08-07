//! Despawn inactive level roots per [`LodScene::scene_lod_culls`].

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::gen::LodScene;
use crate::lod_cull::LodSceneCulls;
use crate::lod_level::LodSceneLevel;
use crate::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use super::bounds::{ephemeral_bounds, LodHostBounds};
use super::node::{
	collect_node_snapshots, dominant_lod_ref, lod_refs_for_bounds, LodNode, LodNodePose,
};

/// Despawn inactive [`LodLevelRoot`]s listed by [`LodScene::scene_lod_culls`].
///
/// Never despawns the host's current [`LodSceneLevel`]. Hidden roots not listed
/// stay warm for cheap band flips.
pub fn cull_lod_level_roots<T, FHost, FNode>(
	mut commands: Commands,
	nodes: Query<(Entity, &LodNodePose), (With<LodNode>, FNode)>,
	hosts: Query<
		(&T, Option<&LodHostBounds>, &LodSceneLevel, &Children),
		(With<LodSceneHost>, FHost),
	>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
) where
	T: Component + LodScene,
	FHost: QueryFilter + 'static,
	FNode: QueryFilter + 'static,
{
	let snapshots = collect_node_snapshots(&nodes);
	if snapshots.is_empty() {
		return;
	}

	let t0 = std::time::Instant::now();
	let mut despawned = 0u32;

	for (scene, host_bounds, current, host_children) in &hosts {
		let bounds = ephemeral_bounds(host_bounds);
		let refs = lod_refs_for_bounds(&snapshots, &bounds);
		let Some(lod_ref) = dominant_lod_ref(scene, &refs) else {
			continue;
		};
		let culls = scene.scene_lod_culls(lod_ref, *current);
		if matches!(culls, LodSceneCulls::None) {
			continue;
		}

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
			let Ok(root) = root_keys.get(child) else {
				continue;
			};
			if root.0 == *current {
				continue;
			}
			if culls.should_cull(root.0) {
				commands.entity(child).despawn();
				despawned += 1;
			}
		}
	}
	let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
	if despawned > 0 || elapsed_ms >= 0.5 {
		info!(
			"[lod.refresh] cull_lod_level_roots: despawned={despawned} in {elapsed_ms:.2}ms"
		);
	}
}
