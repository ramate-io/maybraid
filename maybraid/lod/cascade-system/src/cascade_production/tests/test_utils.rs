//! Shared [`App`] wiring and spawn helpers for cascade production tests.

use anyhow::{anyhow, Result};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod_cascade::{Cascade, Chunk};

use crate::cascade_production::{
	CascadeChunk, CascadeProduction, CascadeProductionPlugin, CascadeProductionSignalMarker,
	CascadeProductionSource, MarkedBounds, RequirementSignal, StandardFlow, StandardRequirement,
};

/// Type tag for [`StandardFlow`] tests (flow A).
#[derive(Debug)]
pub struct FlowAlpha;

/// Type tag for [`StandardFlow`] tests (flow B).
#[derive(Debug)]
pub struct FlowBeta;

pub type AlphaFlow = StandardFlow<FlowAlpha, StandardRequirement>;
pub type BetaFlow = StandardFlow<FlowBeta, StandardRequirement>;

pub fn leaf_only_cascade() -> Cascade {
	Cascade::new(Vec3::ONE, 0, None)
}

pub fn marked_bounds_at_center_half_extents<T: Send + Sync + 'static>(
	center: Vec3,
	half_extents: Vec3,
) -> MarkedBounds<T> {
	MarkedBounds::new(Aabb3d::new(center, half_extents))
}

pub fn app_with_flow<T: Send + Sync + 'static>() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(CascadeProductionPlugin::<StandardFlow<T, StandardRequirement>>::default());
	app
}

pub fn app_alpha_only() -> App {
	app_with_flow::<FlowAlpha>()
}

pub fn app_dual_flow() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(CascadeProductionPlugin::<AlphaFlow>::default());
	app.add_plugins(CascadeProductionPlugin::<BetaFlow>::default());
	app
}

pub fn spawn_standard_producer<T: Send + Sync + 'static>(
	world: &mut World,
	cascade: Cascade,
	marked_bounds: MarkedBounds<T>,
	requirement: StandardRequirement,
) -> Entity {
	world
		.spawn((
			CascadeProduction::<StandardFlow<T, StandardRequirement>>::new(cascade),
			marked_bounds,
			requirement,
		))
		.id()
}

/// Leaf footprint at `focal` for `cascade` (`rings == 0` ⇒ one cube per focal).
pub fn expected_leaf_chunk_for_focal(focal: Vec3, cascade: &Cascade) -> Chunk {
	let o = cascade.leaf_origin(focal);
	Chunk::from_min_max(o, o + cascade.leaf_scale(), None)
}

pub fn chunk_footprint(world: &World, entity: Entity) -> Result<Chunk> {
	world
		.entity(entity)
		.get::<CascadeChunk>()
		.map(|cc| cc.0)
		.ok_or_else(|| anyhow!("entity {entity:?} missing CascadeChunk"))
}

pub fn producer_children<'w>(world: &'w World, producer: Entity) -> Result<&'w Children> {
	world
		.entity(producer)
		.get::<Children>()
		.ok_or_else(|| anyhow!("producer {producer:?} missing Children"))
}

pub fn producer_table_entries<T: Send + Sync + 'static>(
	world: &World,
	producer: Entity,
) -> Result<Vec<(Chunk, Entity)>> {
	let prod = world
		.entity(producer)
		.get::<CascadeProduction<StandardFlow<T, StandardRequirement>>>()
		.ok_or_else(|| anyhow!("producer {producer:?} missing CascadeProduction"))?;
	Ok(prod.table.table.iter().map(|(&chunk, &entity)| (chunk, entity)).collect())
}

pub fn producer_chunk_table_len<T: Send + Sync + 'static>(
	world: &World,
	producer: Entity,
) -> Result<usize> {
	let prod = world
		.entity(producer)
		.get::<CascadeProduction<StandardFlow<T, StandardRequirement>>>()
		.ok_or_else(|| anyhow!("producer {producer:?} missing CascadeProduction"))?;
	Ok(prod.table.table.len())
}

pub fn producer_first_chunk_entity<T: Send + Sync + 'static>(
	world: &World,
	producer: Entity,
) -> Result<Entity> {
	let prod = world
		.entity(producer)
		.get::<CascadeProduction<StandardFlow<T, StandardRequirement>>>()
		.ok_or_else(|| anyhow!("producer {producer:?} missing CascadeProduction"))?;
	let (&_chunk, &entity) = prod
		.table
		.table
		.iter()
		.next()
		.ok_or_else(|| anyhow!("producer {producer:?} chunk table unexpectedly empty"))?;
	Ok(entity)
}

pub fn typed_signal_count<S: CascadeProductionSource>(world: &mut World) -> usize {
	world
		.query_filtered::<Entity, (
			With<CascadeChunk>,
			With<RequirementSignal>,
			With<CascadeProductionSignalMarker<S>>,
		)>()
		.iter(world)
		.count()
}

pub fn spawn_orphan_signal<T: Send + Sync + 'static>(
	world: &mut World,
	chunk: Chunk,
	signal: RequirementSignal,
) -> Entity {
	world
		.spawn((
			CascadeChunk(chunk),
			signal,
			MarkedBounds::<T>::signal_placeholder(),
			CascadeProductionSignalMarker::<StandardFlow<T, StandardRequirement>>::default(),
		))
		.id()
}

pub fn chunk_entity_alive(world: &World, entity: Entity) -> bool {
	world.get_entity(entity).is_ok()
}
