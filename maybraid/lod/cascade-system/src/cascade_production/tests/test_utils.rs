//! Shared [`App`] wiring and spawn helpers for cascade production tests.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod_cascade::{Cascade, Chunk};

use super::super::{
	CascadeChunk, CascadeProduction, CascadeProductionPlugin, CascadeProductionSignalMarker,
	CascadeProductionSource, RequirementSignal, StandardBounds, StandardFlow, StandardMarker,
	StandardRequirement,
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

pub fn bounds_from_center_half_extents(center: Vec3, half_extents: Vec3) -> StandardBounds {
	StandardBounds(Aabb3d::new(center, half_extents))
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
	bounds: StandardBounds,
) -> Entity {
	world
		.spawn((
			CascadeProduction::<StandardFlow<T, StandardRequirement>>::new(cascade),
			bounds,
			StandardMarker::<T>::default(),
			StandardRequirement,
		))
		.id()
}

pub fn producer_chunk_table_len<T: Send + Sync + 'static>(
	world: &World,
	producer: Entity,
) -> usize {
	world
		.entity(producer)
		.get::<CascadeProduction<StandardFlow<T, StandardRequirement>>>()
		.expect("producer missing CascadeProduction")
		.table
		.table
		.len()
}

pub fn producer_first_chunk_entity<T: Send + Sync + 'static>(
	world: &World,
	producer: Entity,
) -> Entity {
	let prod = world
		.entity(producer)
		.get::<CascadeProduction<StandardFlow<T, StandardRequirement>>>()
		.expect("producer missing CascadeProduction");
	let (&_chunk, &entity) = prod.table.table.iter().next().expect("non-empty table");
	entity
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
			StandardMarker::<T>::default(),
			CascadeProductionSignalMarker::<StandardFlow<T, StandardRequirement>>::default(),
		))
		.id()
}

pub fn chunk_entity_alive(world: &World, entity: Entity) -> bool {
	world.get_entity(entity).is_ok()
}
