//! Region-driven LOD refresh: stamp [`LodRefresh`] from [`LodSceneRefreshRegions`],
//! then run the fine-pass sync pipeline only on marked hosts.
//!
//! - [`LodRefresh::Fine`]: ongoing (kept after the pass).
//! - [`LodRefresh::Coarse`]: one-shot (cleared after Cull).
//!
//! Region lookup is generic over [`LodSceneRegionIndex`] implementers via
//! [`StaticSystemParam`] so plugins can plug Avian (or another broadphase).

use std::collections::HashSet;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::fine_pass::{
	configure_fine_pass_sets, ephemeral_bounds, fulfill_lod_level_spawn, LodFinePassSystems,
	LodHostBounds, LodViewerState,
};
use crate::gen::LodScene;
use crate::lod_cull::LodSceneCulls;
use crate::lod_level::LodSceneLevel;
use crate::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use crate::region_index::LodSceneRegionIndex;

/// Cascade / producer output: AABBs that should participate in LOD refresh.
#[derive(Debug, Clone, Default, Component)]
pub struct LodSceneRefreshRegions {
	pub fine: Vec<Aabb3d>,
	pub coarse: Vec<Aabb3d>,
}

/// Behavioral marker: host is in the refresh work set for this pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub enum LodRefresh {
	/// Inner ring — keep after the sync pass (re-evaluate every frame).
	Fine,
	/// Outer / tile reload — clear after the sync pass (one-shot).
	Coarse,
}

/// Extra fine-pass steps for region-driven refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodRefreshSystems {
	/// Stamp [`LodRefresh`] from [`LodSceneRefreshRegions`] (on change).
	Mark,
	/// Drop [`LodRefresh::Coarse`] after Cull.
	ClearCoarse,
}

pub(crate) fn configure_refresh_sets(app: &mut App) {
	configure_fine_pass_sets(app);
	app.configure_sets(
		Update,
		LodRefreshSystems::Mark
			.after(LodFinePassSystems::Track)
			.before(LodFinePassSystems::UpdateLevels),
	);
	app.configure_sets(
		Update,
		LodRefreshSystems::ClearCoarse.after(LodFinePassSystems::Cull),
	);
}

/// On [`Changed<LodSceneRefreshRegions>`], assign [`LodRefresh`] from region hits.
///
/// Fine wins over coarse when an entity appears in both. Hosts of type `T` that
/// leave all listed regions lose their marker.
pub fn mark_lod_refresh_from_regions<T, I>(
	index: StaticSystemParam<I>,
	regions_q: Query<&LodSceneRefreshRegions, Changed<LodSceneRefreshRegions>>,
	mut commands: Commands,
	existing: Query<(Entity, Option<&LodRefresh>), (With<LodSceneHost>, With<T>)>,
) where
	T: Component + LodScene + 'static,
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
{
	let index = index.into_inner();
	for regions in &regions_q {
		let mut fine: HashSet<Entity> = HashSet::new();
		let mut coarse: HashSet<Entity> = HashSet::new();

		for region in &regions.fine {
			for (entity, _) in index.hosts_in_region(*region) {
				fine.insert(entity);
			}
		}
		for region in &regions.coarse {
			for (entity, _) in index.hosts_in_region(*region) {
				if !fine.contains(&entity) {
					coarse.insert(entity);
				}
			}
		}

		for (entity, prev) in &existing {
			if fine.contains(&entity) {
				if !matches!(prev, Some(LodRefresh::Fine)) {
					commands.entity(entity).insert(LodRefresh::Fine);
				}
			} else if coarse.contains(&entity) {
				if !matches!(prev, Some(LodRefresh::Coarse)) {
					commands.entity(entity).insert(LodRefresh::Coarse);
				}
			} else if prev.is_some() {
				commands.entity(entity).remove::<LodRefresh>();
			}
		}
	}
}

/// Like [`crate::update_lod_host_levels`], but only hosts with [`LodRefresh`].
pub fn update_lod_host_levels_refresh<T: Component + LodScene>(
	viewer: Res<LodViewerState>,
	mut hosts: Query<
		(&T, &LodHostBounds, &mut LodSceneLevel),
		(With<LodSceneHost>, With<LodRefresh>),
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

/// Like [`crate::cull_lod_level_roots`], but only hosts with [`LodRefresh`].
pub fn cull_lod_level_roots_refresh<T: Component + LodScene>(
	viewer: Res<LodViewerState>,
	mut commands: Commands,
	hosts: Query<
		(&T, Option<&LodHostBounds>, &LodSceneLevel, &Children),
		(With<LodSceneHost>, With<LodRefresh>),
	>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
) {
	if viewer.entity == Entity::PLACEHOLDER {
		return;
	}

	let t0 = std::time::Instant::now();
	let mut despawned = 0u32;

	for (scene, host_bounds, current, host_children) in &hosts {
		let bounds = ephemeral_bounds(host_bounds);
		let lod_ref = viewer.lod_ref(&bounds);
		let culls = scene.scene_lod_culls(&lod_ref, *current);
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

/// Remove one-shot [`LodRefresh::Coarse`] after the sync pass completes.
pub fn clear_coarse_lod_refresh<T: Component + LodScene>(
	mut commands: Commands,
	hosts: Query<(Entity, &LodRefresh), (With<LodSceneHost>, With<T>)>,
) {
	for (entity, refresh) in &hosts {
		if matches!(refresh, LodRefresh::Coarse) {
			commands.entity(entity).remove::<LodRefresh>();
		}
	}
}

/// Register region-driven refresh + fine-pass sync for one host type and index param.
///
/// `I` is a [`SystemParam`] whose item implements [`LodSceneRegionIndex<T>`]
/// (e.g. `AvianLodSceneRegionIndex<'_, '_, T>`).
///
/// Still requires [`crate::LodFinePassPlugin`] for viewer track + root sync.
pub fn add_lod_scene_refresh_for<T, I>(app: &mut App)
where
	T: Component + LodScene + 'static,
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
{
	configure_refresh_sets(app);
	app.add_systems(
		Update,
		(
			mark_lod_refresh_from_regions::<T, I>.in_set(LodRefreshSystems::Mark),
			update_lod_host_levels_refresh::<T>.in_set(LodFinePassSystems::UpdateLevels),
			fulfill_lod_level_spawn::<T>.in_set(LodFinePassSystems::Fulfill),
			cull_lod_level_roots_refresh::<T>.in_set(LodFinePassSystems::Cull),
			clear_coarse_lod_refresh::<T>.in_set(LodRefreshSystems::ClearCoarse),
		),
	);
}
