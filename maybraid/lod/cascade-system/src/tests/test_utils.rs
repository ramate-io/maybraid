//! Shared fixtures for crate-level integration tests.

use anyhow::{anyhow, Result};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod_cascade::{Cascade, Chunk};

use crate::cascade_production::{
	CascadeChunk, CascadeProduction, CascadeProductionSignalMarker, CascadeProductionSource,
	MarkedBounds, RequirementSignal, StandardFlow, StandardRequirement, TrackBounds,
};
use crate::chunk_entity_tracker::ChunkEntityPosition;
use crate::chunk_tracker::ChunkTracker;

/// Flow tag for [`IntegrationFlow`] (examples / integration tests).
#[derive(Debug)]
pub struct IntegrationTag;

/// Every-frame production (`QueryFilter = ()`) so multi-tick scenarios do not depend on
/// `Changed<MarkedBounds<_>>`.
pub type IntegrationFlow = StandardFlow<IntegrationTag, StandardRequirement, ()>;

/// Observations recorded by [`RecordingChunkTracker`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackerObservation {
	pub chunk: Chunk,
	pub signal: RequirementSignal,
}

/// Example [`ChunkTracker`] that spawns a marker entity per reaction (inspect with a query).
pub struct RecordingChunkTracker;

impl ChunkTracker<IntegrationFlow> for RecordingChunkTracker {
	fn react(commands: &mut Commands, chunk: Chunk, signal: RequirementSignal) {
		commands.spawn(TrackerObservation { chunk, signal });
	}
}

pub fn leaf_only_cascade() -> Cascade {
	Cascade::new(Vec3::ONE, 0, None)
}

pub fn marked_bounds_at_center_half_extents(
	center: Vec3,
	half_extents: Vec3,
) -> MarkedBounds<IntegrationTag> {
	MarkedBounds::new(Aabb3d::new(center, half_extents))
}

pub fn spawn_integration_producer(
	world: &mut World,
	cascade: Cascade,
	marked_bounds: MarkedBounds<IntegrationTag>,
	requirement: StandardRequirement,
) -> Entity {
	world
		.spawn((CascadeProduction::<IntegrationFlow>::new(cascade), marked_bounds, requirement))
		.id()
}

/// Recenters producer focal to `new_center` (same half-extents as initial fixtures: **`10`**).
pub fn integration_leaf_bounds_recenter(
	world: &mut World,
	producer: Entity,
	new_center: Vec3,
) -> Result<()> {
	let bounds = marked_bounds_at_center_half_extents(new_center, Vec3::splat(10.0));
	world.entity_mut(producer).insert(bounds);
	Ok(())
}

pub fn expected_leaf_chunk_for_focal(focal: Vec3, cascade: &Cascade) -> Chunk {
	let o = cascade.leaf_origin(focal);
	Chunk::from_min_max(o, o + cascade.leaf_scale(), None)
}

pub fn observation_count(world: &mut World) -> usize {
	world.query_filtered::<Entity, With<TrackerObservation>>().iter(world).count()
}

/// Bounds for [`crate::chunk_entity_tracker::track_chunk_entities`] integration examples.
#[derive(Component, Clone)]
pub struct ManagedEntityBounds {
	pub previous: Option<TrackBounds>,
	pub current: TrackBounds,
}

impl ChunkEntityPosition<IntegrationFlow> for ManagedEntityBounds {
	fn previous(&self) -> Option<TrackBounds> {
		self.previous
	}

	fn current(&self) -> TrackBounds {
		self.current
	}
}

pub fn spawn_managed_under_chunk(
	world: &mut World,
	chunk: Entity,
	bounds: ManagedEntityBounds,
) -> Entity {
	world.spawn((bounds, ChildOf(chunk))).id()
}

pub fn parent_of_child(world: &World, child: Entity) -> Result<Entity> {
	let child_of = world
		.entity(child)
		.get::<ChildOf>()
		.ok_or_else(|| anyhow!("entity {child:?} missing ChildOf"))?;
	Ok(child_of.parent())
}

pub fn requirement_signal_entity_count<S: CascadeProductionSource>(world: &mut World) -> usize {
	world
		.query_filtered::<Entity, (
			With<CascadeChunk>,
			With<RequirementSignal>,
			With<CascadeProductionSignalMarker<S>>,
		)>()
		.iter(world)
		.count()
}

/// Drives opt-in GC tests: keep **`0`** while signals must survive a tick, then set **`> 0`** so
/// [`garbage_collect_requirement_signals`](crate::cascade_production::garbage_collect_requirement_signals)
/// runs (see [`gc_run_condition`]).
#[derive(Resource, Default)]
pub struct GcCounter(pub u32);

pub fn gc_run_condition(c: Res<GcCounter>) -> bool {
	c.0 > 0
}

pub fn aabb_center_half(center: Vec3, half: f32) -> TrackBounds {
	Aabb3d::new(center, Vec3::splat(half))
}

pub fn chunk_entity_for_footprint(
	world: &World,
	producer: Entity,
	footprint: Chunk,
) -> Result<Entity> {
	let prod = world
		.entity(producer)
		.get::<CascadeProduction<IntegrationFlow>>()
		.ok_or_else(|| anyhow!("producer missing CascadeProduction"))?;
	prod.table
		.table
		.get(&footprint)
		.copied()
		.ok_or_else(|| anyhow!("chunk not in producer table"))
}
