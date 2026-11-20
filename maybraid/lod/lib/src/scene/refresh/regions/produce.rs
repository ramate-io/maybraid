//! Produce [`LodSceneRefreshRegion`] messages from [`LodNode`] drivers.

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use bevy::shape::Aabb3d;
use thiserror::Error;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose, LodRef,
};

use super::super::levels::{LodRefreshDomain, LodSceneRefreshAabb};
use super::super::{ensure_refresh_core, LodRefreshSystems};

/// Impulse: refresh hosts overlapping `region` (type `M` scopes the channel).
#[derive(Message, Debug, Clone)]
pub struct LodSceneRefreshRegion<M: Send + Sync + 'static> {
	pub region: Aabb3d,
	pub _marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> LodSceneRefreshRegion<M> {
	pub fn new(region: Aabb3d) -> Self {
		Self { region, _marker: PhantomData }
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

	/// Region this driver covers now, whether or not it just crossed a threshold.
	///
	/// Generation producers union this over every active driver, so a resting
	/// driver keeps its coverage while another one moves. The default evaluates
	/// [`Self::lod_refresh_regions`] as if the driver had arrived from nowhere;
	/// override it when that is not the driver's current region.
	fn lod_coverage(&self, lod_ref: &LodRef) -> Option<Aabb3d> {
		let nowhere = Transform::from_translation(Vec3::INFINITY);
		let arrived = LodRef { previous_transform: &nowhere, ..*lod_ref };
		match self.lod_refresh_regions(&arrived) {
			LodRefreshRegionsStatus::Changed(region) => Some(region),
			LodRefreshRegionsStatus::Unchanged => None,
		}
	}

	/// Union of [`Self::lod_coverage`] over every driver.
	fn lod_coverage_union(&self, lod_refs: &[LodRef]) -> Option<Aabb3d> {
		lod_refs
			.iter()
			.filter_map(|lod_ref| self.lod_coverage(lod_ref))
			.reduce(union_aabb)
	}

	/// Fold many drivers into one max-extent AABB (or [`Unchanged`] / [`Empty`]).
	///
	/// A moving driver still unions [`Self::lod_coverage`] of every resting
	/// driver, so a ground [`LodRef`] keeps its neighborhood while a map camera
	/// climbs away.
	fn lod_refresh_regions_for(
		&self,
		lod_refs: &[LodRef],
	) -> Result<LodRefreshRegionsStatus, LodRefreshRegionsError> {
		if lod_refs.is_empty() {
			return Err(LodRefreshRegionsError::Empty);
		}

		let mut any_changed = false;
		let mut union: Option<Aabb3d> = None;
		for lod_ref in lod_refs {
			match self.lod_refresh_regions(lod_ref) {
				LodRefreshRegionsStatus::Unchanged => {
					if let Some(aabb) = self.lod_coverage(lod_ref) {
						union = Some(match union {
							None => aabb,
							Some(prev) => union_aabb(prev, aabb),
						});
					}
				}
				LodRefreshRegionsStatus::Changed(aabb) => {
					any_changed = true;
					union = Some(match union {
						None => aabb,
						Some(prev) => union_aabb(prev, aabb),
					});
				}
			}
		}

		Ok(if any_changed {
			match union {
				Some(region) => LodRefreshRegionsStatus::Changed(region),
				None => LodRefreshRegionsStatus::Unchanged,
			}
		} else {
			LodRefreshRegionsStatus::Unchanged
		})
	}
}

fn union_aabb(a: Aabb3d, b: Aabb3d) -> Aabb3d {
	Aabb3d::from_min_max(a.min.min(b.min), a.max.max(b.max))
}

/// Read `F`-filtered [`LodNode`]s, compute a region via `P`, write
/// [`LodSceneRefreshRegion<M>`].
///
/// Pose / bounds / membership changes trigger a refresh. Coverage then
/// unions every current driver, so a resting [`LodRef`] keeps its neighborhood
/// while another one moves. Strategies still return
/// [`LodRefreshRegionsStatus::Unchanged`] for sub-threshold motion; adding or
/// removing a settled driver still emits the current union.
pub fn produce_lod_refresh_regions<P, F, M>(
	producer: Res<P>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	changed_pose: Query<Entity, (With<LodNode>, F, Changed<LodNodePose>)>,
	changed_bounds: Query<Entity, (With<LodNode>, F, Changed<LodNodeBounds>)>,
	mut writer: MessageWriter<LodSceneRefreshRegion<M>>,
	mut bus: MessageWriter<LodSceneRefreshAabb>,
	mut members: Local<Vec<Entity>>,
) where
	P: Resource + LodRefreshRegions,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	if nodes.is_empty() {
		members.clear();
		return;
	}
	let mut current: Vec<Entity> = nodes.iter().map(|(entity, _, _)| entity).collect();
	current.sort_unstable();
	let membership_changed = *members != current;
	let drivers_changed = !changed_pose.is_empty() || !changed_bounds.is_empty();
	if !membership_changed && !drivers_changed {
		return;
	}
	members.clone_from(&current);

	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);
	let region = match producer.lod_refresh_regions_for(&refs) {
		Ok(LodRefreshRegionsStatus::Changed(region)) => region,
		Ok(LodRefreshRegionsStatus::Unchanged) if membership_changed => {
			let Some(region) = producer.lod_coverage_union(&refs) else {
				return;
			};
			region
		}
		_ => return,
	};

	writer.write(LodSceneRefreshRegion::<M>::new(region));
	bus.write(LodSceneRefreshAabb { region, domain: LodRefreshDomain::of::<M>() });
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
		Self { _marker: PhantomData }
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
			.add_message::<LodSceneRefreshAabb>()
			.add_systems(
				Update,
				produce_lod_refresh_regions::<P, F, M>.in_set(LodRefreshSystems::ProduceRegions),
			);
	}
}
