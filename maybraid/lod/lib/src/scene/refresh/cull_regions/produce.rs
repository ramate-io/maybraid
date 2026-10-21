//! Produce [`LodSceneCullRegion`] messages from [`LodNode`] drivers.

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose, LodRef,
};

use super::super::{ensure_refresh_core, LodCullProduceSystems};
use super::cache::LodSceneCullAabb;
use super::cursor::{
	LodCullAabbThisTick, LodCullProduceCadence, LodCullProduceClock, LodCullRegionCursor,
};

/// Advance the shared cull cadence and clear this-frame AABBs before any channel emits.
pub fn tick_lod_cull_produce_clock(
	cadence: Res<LodCullProduceCadence>,
	mut clock: ResMut<LodCullProduceClock>,
	mut batch: ResMut<LodCullAabbThisTick>,
) {
	clock.tick(*cadence);
	batch.clear();
}

/// Impulse: cull-evaluate hosts overlapping `region` (channel `M`).
#[derive(Message, Debug, Clone)]
pub struct LodSceneCullRegion<M: Send + Sync + 'static> {
	pub region: Aabb3d,
	pub _marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> LodSceneCullRegion<M> {
	pub fn new(region: Aabb3d) -> Self {
		Self { region, _marker: PhantomData }
	}
}

/// Result of mapping driver [`LodRef`]s + cursor to cull region AABBs.
#[derive(Debug, Clone)]
pub enum LodCullRegionsStatus {
	/// Emit nothing this tick.
	Unchanged,
	/// One or more tile / ring AABBs (usually a single rotating cell).
	Changed(Vec<Aabb3d>),
}

/// How to produce cull region AABBs from drivers + a rotating [`LodCullRegionCursor`].
///
/// Unlike [`super::super::regions::LodRefreshRegions`], this may emit every frame
/// (camera still) and returns a **batch** of regions rather than one union AABB.
pub trait LodCullRegions: Send + Sync + 'static {
	fn lod_cull_regions(
		&self,
		lod_refs: &[LodRef],
		cursor: &mut LodCullRegionCursor,
	) -> LodCullRegionsStatus;
}

/// Read `F`-filtered [`LodNode`]s (not pose-change gated), advance `P` + cursor,
/// write [`LodSceneCullRegion<M>`].
pub fn produce_lod_cull_regions<P, F, M>(
	producer: Res<P>,
	mut cursor: ResMut<LodCullRegionCursor>,
	clock: Res<LodCullProduceClock>,
	mut batch: ResMut<LodCullAabbThisTick>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut writer: MessageWriter<LodSceneCullRegion<M>>,
	mut bus: MessageWriter<LodSceneCullAabb>,
) where
	P: Resource + LodCullRegions,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	if nodes.is_empty() {
		return;
	}
	if !clock.emit_this_frame {
		return;
	}

	if batch.regions.is_empty() && !batch.wrote_untyped {
		if producer.is_changed() {
			cursor.invalidate_cells();
		}
		let snapshots = collect_node_snapshots(&nodes);
		let refs = lod_refs_from_snapshots(&snapshots);
		let LodCullRegionsStatus::Changed(regions) = producer.lod_cull_regions(&refs, &mut cursor)
		else {
			batch.wrote_untyped = true;
			return;
		};
		batch.regions = regions;
	}

	for region in batch.regions.iter().copied() {
		writer.write(LodSceneCullRegion::<M>::new(region));
		if !batch.wrote_untyped {
			bus.write(LodSceneCullAabb { region });
		}
	}
	batch.wrote_untyped = true;
}

/// Produce [`LodSceneCullRegion<M>`] via strategy `P` + shared [`LodCullRegionCursor`].
pub struct LodSceneCullRegionPlugin<P, F, M>
where
	P: Resource + LodCullRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	_marker: PhantomData<fn() -> (P, F, M)>,
}

impl<P, F, M> Default for LodSceneCullRegionPlugin<P, F, M>
where
	P: Resource + LodCullRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<P, F, M> Plugin for LodSceneCullRegionPlugin<P, F, M>
where
	P: Resource + LodCullRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.init_resource::<P>()
			.init_resource::<LodCullRegionCursor>()
			.init_resource::<LodCullProduceCadence>()
			.init_resource::<LodCullProduceClock>()
			.init_resource::<LodCullAabbThisTick>()
			.add_message::<LodSceneCullRegion<M>>()
			.add_message::<LodSceneCullAabb>()
			.add_systems(
				Update,
				produce_lod_cull_regions::<P, F, M>.in_set(LodCullProduceSystems::Emit),
			);
	}
}
