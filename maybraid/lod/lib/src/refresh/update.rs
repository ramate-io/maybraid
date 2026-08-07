//! Write desired [`LodSceneLevel`] on hosts.

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::gen::LodScene;
use crate::lod_level::LodSceneLevel;
use crate::lod_scene_host::LodSceneHost;

use super::bounds::LodHostBounds;
use super::viewer::LodViewerState;

/// Set host [`LodSceneLevel`] from [`LodViewerState`] + [`LodHostBounds`].
///
/// `F` scopes the host query (`()` = all hosts; `With<LodRefresh>` for marked refresh).
pub fn update_lod_host_levels<T: Component + LodScene, F: QueryFilter + 'static>(
	viewer: Res<LodViewerState>,
	mut hosts: Query<
		(&T, &LodHostBounds, &mut LodSceneLevel),
		(With<LodSceneHost>, F),
	>,
) {
	if viewer.entity == Entity::PLACEHOLDER {
		return;
	}
	let t0 = std::time::Instant::now();
	let mut changed = 0u32;
	let mut n = 0u32;
	for (scene, bounds, mut level) in &mut hosts {
		n += 1;
		let lod_ref = viewer.lod_ref(&bounds.0);
		let desired = scene.scene_lod_level(&lod_ref);
		if *level != desired {
			*level = desired;
			changed += 1;
		}
	}
	let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
	if changed > 0 || elapsed_ms >= 0.5 {
		info!(
			"[lod.refresh] update_lod_host_levels: hosts={n} changed={changed} in {elapsed_ms:.2}ms"
		);
	}
}
