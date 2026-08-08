//! Produce [`LodSceneRefreshRegion`] messages from [`LodNode`] drivers.

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use thiserror::Error;

use crate::lod_ref::{collect_node_snapshots, LodNode, LodNodePose, LodNodeSnapshot, LodRef};

use super::super::{ensure_refresh_core, LodRefreshSystems};

/// Impulse: refresh hosts overlapping `region` (type `M` scopes the channel).
#[derive(Message, Debug, Clone)]
pub struct LodSceneRefreshRegion<M: Send + Sync + 'static> {
	pub region: Aabb3d,
	pub _marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> LodSceneRefreshRegion<M> {
	pub fn new(region: Aabb3d) -> Self {
		Self {
			region,
			_marker: PhantomData,
		}
	}
}

/// Result of mapping driver [`LodRef`]s to a refresh region.
#[derive(Debug, Clone)]
pub enum LodRefreshRegionsStatus {
	/// Driver did not cross a production threshold; emit nothing.
	Unchanged,
	/// Emit a max-extent AABB.
	Changed(Aabb3d),
}

/// Errors from [`LodRefreshRegions::lod_refresh_regions_for`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LodRefreshRegionsError {
	/// No driver [`LodRef`]s were provided.
	#[error("lod_refresh_regions_for called with no LodRefs")]
	Empty,
}

/// How to produce a max-extent refresh AABB from driver [`LodRef`]s.
///
/// Implementors are typically registered as a [`Resource`] and driven by
/// [`LodSceneRefreshRegionPlugin`].
pub trait LodRefreshRegions: Send + Sync + 'static {
	/// Region for a single driver. Must be cheap — no scene build.
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus;

	/// Fold many drivers into one max-extent AABB (or [`Unchanged`] / [`Empty`]).
	fn lod_refresh_regions_for(
		&self,
		lod_refs: &[&LodRef],
	) -> Result<LodRefreshRegionsStatus, LodRefreshRegionsError> {
		if lod_refs.is_empty() {
			return Err(LodRefreshRegionsError::Empty);
		}

		let mut union: Option<Aabb3d> = None;
		for lod_ref in lod_refs {
			match self.lod_refresh_regions(lod_ref) {
				LodRefreshRegionsStatus::Unchanged => {}
				LodRefreshRegionsStatus::Changed(aabb) => {
					union = Some(match union {
						None => aabb,
						Some(prev) => union_aabb(prev, aabb),
					});
				}
			}
		}

		Ok(match union {
			Some(region) => LodRefreshRegionsStatus::Changed(region),
			None => LodRefreshRegionsStatus::Unchanged,
		})
	}
}

fn union_aabb(a: Aabb3d, b: Aabb3d) -> Aabb3d {
	Aabb3d::from_min_max(a.min.min(b.min), a.max.max(b.max))
}

/// Point AABB at a node's current translation (drivers have no host bounds).
fn snapshot_bounds(snapshot: &LodNodeSnapshot) -> Aabb3d {
	let p = snapshot.current.translation;
	Aabb3d::from_min_max(p, p)
}

/// Read `F`-filtered [`LodNode`]s, compute a region via `P`, write [`LodSceneRefreshRegion<M>`].
pub fn produce_lod_refresh_regions<P, F, M>(
	producer: Res<P>,
	nodes: Query<(Entity, &LodNodePose), (With<LodNode>, F)>,
	mut writer: MessageWriter<LodSceneRefreshRegion<M>>,
) where
	P: Resource + LodRefreshRegions,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
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

	let Ok(LodRefreshRegionsStatus::Changed(region)) =
		producer.lod_refresh_regions_for(&ref_refs)
	else {
		return;
	};

	writer.write(LodSceneRefreshRegion::<M>::new(region));
}

/// Produce [`LodSceneRefreshRegion<M>`] from `F`-filtered [`LodNode`]s via strategy `P`.
///
/// `P` is a [`Resource`] implementing [`LodRefreshRegions`] (`init_resource` on add).
/// `M` is a channel marker (`Send + Sync`), not an ECS component.
pub struct LodSceneRefreshRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	_marker: PhantomData<fn() -> (P, F, M)>,
}

impl<P, F, M> Default for LodSceneRefreshRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<P, F, M> Plugin for LodSceneRefreshRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.init_resource::<P>()
			.add_message::<LodSceneRefreshRegion<M>>()
			.add_systems(
				Update,
				produce_lod_refresh_regions::<P, F, M>.in_set(LodRefreshSystems::ProduceRegions),
			);
	}
}
