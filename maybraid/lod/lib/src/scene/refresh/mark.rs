//! Stamp [`LodRefresh`] from marker-scoped [`LodSceneRefreshRegions`].

use std::collections::HashSet;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::scene::LodScene;
use crate::scene::host::LodSceneHost;
use crate::scene::region_index::LodSceneRegionIndex;

/// Cascade / producer output: AABBs that should participate in LOD refresh.
///
/// Tag the producer entity with marker `M`; host type `T` listens via
/// [`crate::LodBroadPhasePlugin`] / [`crate::scene::LodSceneRefreshPlugin`].
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

/// On change of any `M`-tagged [`LodSceneRefreshRegions`], re-stamp [`LodRefresh`].
///
/// Unions all current `M` region lists (not only the changed entity) so sibling
/// producers with the same marker stay consistent. Fine wins over coarse.
pub fn mark_lod_refresh_from_regions<T, M, I>(
	index: StaticSystemParam<I>,
	changed: Query<(), (With<M>, Changed<LodSceneRefreshRegions>)>,
	regions_q: Query<&LodSceneRefreshRegions, With<M>>,
	mut commands: Commands,
	existing: Query<(Entity, Option<&LodRefresh>), (With<LodSceneHost>, With<T>)>,
) where
	T: Component + LodScene + 'static,
	M: Component + 'static,
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
{
	if changed.is_empty() {
		return;
	}

	let index = index.into_inner();
	let mut fine: HashSet<Entity> = HashSet::new();
	let mut coarse: HashSet<Entity> = HashSet::new();

	for regions in &regions_q {
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
