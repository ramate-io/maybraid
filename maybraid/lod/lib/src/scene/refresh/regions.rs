//! Produce [`LodSceneRefreshRegions`] from [`LodNode`] drivers.
//!
//! [`LodRefreshRegions`] describes how a strategy maps [`LodRef`]s → regions.
//! [`produce_lod_refresh_regions`] listens to nodes filtered by `F` and upserts a
//! stable outlet entity tagged `M` (via [`Local`] + [`LodRefreshRegionsOutlet`]).

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use thiserror::Error;

use crate::lod_ref::{collect_node_snapshots, LodNode, LodNodePose, LodNodeSnapshot, LodRef};

use super::mark::LodSceneRefreshRegions;

/// Result of computing refresh regions for one or more [`LodRef`]s.
#[derive(Debug, Clone)]
pub enum LodRefreshRegionsStatus {
	/// Regions are unchanged; do not write the outlet (no [`Changed`] trigger).
	Unchanged,
	/// Replace the outlet's [`LodSceneRefreshRegions`].
	Changed(LodSceneRefreshRegions),
}

/// Errors from [`LodRefreshRegions::lod_refresh_regions_for`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LodRefreshRegionsError {
	/// No driver [`LodRef`]s were provided.
	#[error("lod_refresh_regions_for called with no LodRefs")]
	Empty,
}

/// How to produce [`LodSceneRefreshRegions`] from driver [`LodRef`]s.
///
/// Implementors are typically registered as a [`Resource`] and driven by
/// [`super::LodRefreshProductionPlugin`].
pub trait LodRefreshRegions: Send + Sync + 'static {
	/// Regions for a single driver. Must be cheap — no scene build.
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus;

	/// Regions for multiple drivers.
	///
	/// Default: merge unique fine/coarse AABBs from each
	/// [`Self::lod_refresh_regions`]. All [`LodRefreshRegionsStatus::Unchanged`]
	/// → [`LodRefreshRegionsStatus::Unchanged`]. No drivers →
	/// [`LodRefreshRegionsError::Empty`] (no spurious [`Changed`]).
	fn lod_refresh_regions_for(
		&self,
		lod_refs: &[&LodRef],
	) -> Result<LodRefreshRegionsStatus, LodRefreshRegionsError> {
		if lod_refs.is_empty() {
			return Err(LodRefreshRegionsError::Empty);
		}

		let mut any_changed = false;
		let mut fine: Vec<Aabb3d> = Vec::new();
		let mut coarse: Vec<Aabb3d> = Vec::new();

		for lod_ref in lod_refs {
			match self.lod_refresh_regions(lod_ref) {
				LodRefreshRegionsStatus::Unchanged => {}
				LodRefreshRegionsStatus::Changed(regions) => {
					any_changed = true;
					for aabb in regions.fine {
						push_unique_aabb(&mut fine, aabb);
					}
					for aabb in regions.coarse {
						push_unique_aabb(&mut coarse, aabb);
					}
				}
			}
		}

		Ok(if any_changed {
			LodRefreshRegionsStatus::Changed(LodSceneRefreshRegions { fine, coarse })
		} else {
			LodRefreshRegionsStatus::Unchanged
		})
	}
}

fn push_unique_aabb(out: &mut Vec<Aabb3d>, aabb: Aabb3d) {
	if !out.iter().any(|existing| aabbs_equal(existing, &aabb)) {
		out.push(aabb);
	}
}

fn aabbs_equal(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min == b.min && a.max == b.max
}

fn regions_equal(a: &LodSceneRefreshRegions, b: &LodSceneRefreshRegions) -> bool {
	a.fine.len() == b.fine.len()
		&& a.coarse.len() == b.coarse.len()
		&& a.fine.iter().zip(b.fine.iter()).all(|(x, y)| aabbs_equal(x, y))
		&& a.coarse.iter().zip(b.coarse.iter()).all(|(x, y)| aabbs_equal(x, y))
}

/// Marker on the stable outlet entity for one `(P, F, M)` producer channel.
#[derive(Component)]
pub struct LodRefreshRegionsOutlet<P, F, M>
where
	P: LodRefreshRegions,
	F: 'static,
	M: Component,
{
	_marker: PhantomData<fn() -> (P, F, M)>,
}

impl<P, F, M> Default for LodRefreshRegionsOutlet<P, F, M>
where
	P: LodRefreshRegions,
	F: 'static,
	M: Component,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

/// Point AABB at a node's current translation (drivers have no host bounds).
fn snapshot_bounds(snapshot: &LodNodeSnapshot) -> Aabb3d {
	let p = snapshot.current.translation;
	Aabb3d::from_min_max(p, p)
}

/// Read `F`-filtered [`LodNode`]s, compute regions via `P`, upsert outlet with `M`.
pub fn produce_lod_refresh_regions<P, F, M>(
	producer: Res<P>,
	mut outlet: Local<Option<Entity>>,
	mut commands: Commands,
	nodes: Query<(Entity, &LodNodePose), (With<LodNode>, F)>,
	existing_outlets: Query<Entity, (With<LodRefreshRegionsOutlet<P, F, M>>, With<M>)>,
	regions_q: Query<&LodSceneRefreshRegions, With<LodRefreshRegionsOutlet<P, F, M>>>,
) where
	P: Resource + LodRefreshRegions,
	F: QueryFilter + 'static,
	M: Component + Default + 'static,
{
	let snapshots = collect_node_snapshots(&nodes);
	let bounds: Vec<Aabb3d> = snapshots.iter().map(snapshot_bounds).collect();
	let refs: Vec<LodRef> = snapshots
		.iter()
		.zip(bounds.iter())
		.map(|(snap, bounds)| LodRef {
			entity: snap.entity,
			previous_transform: &snap.previous,
			current_transform: &snap.current,
			bounds,
		})
		.collect();
	let ref_refs: Vec<&LodRef> = refs.iter().collect();

	let Ok(LodRefreshRegionsStatus::Changed(regions)) =
		producer.lod_refresh_regions_for(&ref_refs)
	else {
		// `Empty` / `Unchanged`: leave the outlet alone (no spurious Changed).
		return;
	};

	let entity = resolve_outlet::<P, F, M>(&mut *outlet, &mut commands, &existing_outlets);

	if let Ok(current) = regions_q.get(entity) {
		if regions_equal(current, &regions) {
			return;
		}
	}

	commands.entity(entity).insert(regions);
}

fn resolve_outlet<P, F, M>(
	outlet: &mut Option<Entity>,
	commands: &mut Commands,
	existing_outlets: &Query<Entity, (With<LodRefreshRegionsOutlet<P, F, M>>, With<M>)>,
) -> Entity
where
	P: LodRefreshRegions,
	F: 'static,
	M: Component + Default + 'static,
{
	if let Some(entity) = *outlet {
		if existing_outlets.contains(entity) {
			return entity;
		}
	}

	if let Ok(entity) = existing_outlets.single() {
		*outlet = Some(entity);
		return entity;
	}

	let entity = commands
		.spawn((
			LodRefreshRegionsOutlet::<P, F, M>::default(),
			M::default(),
			LodSceneRefreshRegions::default(),
		))
		.id();
	*outlet = Some(entity);
	entity
}
