//! Shared fixtures for chunk-entity tracker tests.

use anyhow::{anyhow, Result};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod_cascade::{Cascade, Chunk};

use crate::cascade_production::{
	CascadeChunk, CascadePosition, CascadeProduction, MarkedBounds, StandardFlow,
	StandardRequirement, TrackBounds,
};
use crate::chunk_entity_tracker::ChunkEntityPosition;

/// Flow tag for [`TestFlow`].
#[derive(Debug)]
pub struct FlowAlpha;

pub type TestFlow = StandardFlow<FlowAlpha, StandardRequirement, ()>;

/// Bounds payload used by [`track_chunk_entities`] tests.
#[derive(Component, Clone)]
pub struct TestEntityBounds {
	pub previous: Option<TrackBounds>,
	pub current: TrackBounds,
}

impl ChunkEntityPosition<TestFlow> for TestEntityBounds {
	fn previous(&self) -> Option<TrackBounds> {
		self.previous
	}

	fn current(&self) -> TrackBounds {
		self.current
	}
}

pub fn leaf_cascade() -> Cascade {
	Cascade::new(Vec3::ONE, 0, None)
}

pub fn adjacent_leaf_chunk_pair() -> (Chunk, Chunk) {
	let a = Chunk::from_min_max(Vec3::ZERO, Vec3::ONE, None);
	let b = Chunk::from_min_max(Vec3::new(1., 0., 0.), Vec3::new(2., 1., 1.), None);
	(a, b)
}

pub fn aabb_center_half(center: Vec3, half: f32) -> TrackBounds {
	Aabb3d::new(center, Vec3::splat(half))
}

/// Producer with two chunk children and both keys in [`CascadeTable`].
pub fn spawn_producer_two_chunks(
	world: &mut World,
	cascade: Cascade,
	chunk_a: Chunk,
	chunk_b: Chunk,
) -> Result<(Entity, Entity, Entity)> {
	let focal = aabb_center_half(Vec3::splat(0.5), 100.0);
	let position = CascadePosition {
		previous: None,
		current: focal,
		data: MarkedBounds::<FlowAlpha>::new(focal),
	};

	let producer = world.spawn((CascadeProduction::<TestFlow>::new(cascade), position)).id();

	let chunk_a_ent = world.spawn((CascadeChunk(chunk_a), Visibility::Visible)).id();
	let chunk_b_ent = world.spawn((CascadeChunk(chunk_b), Visibility::Visible)).id();

	world.entity_mut(producer).add_child(chunk_a_ent);
	world.entity_mut(producer).add_child(chunk_b_ent);

	let mut entity = world.entity_mut(producer);
	let mut prod_mut = entity
		.get_mut::<CascadeProduction<TestFlow>>()
		.ok_or_else(|| anyhow!("producer missing CascadeProduction"))?;
	prod_mut.table.table.insert(chunk_a, chunk_a_ent);
	prod_mut.table.table.insert(chunk_b, chunk_b_ent);

	Ok((producer, chunk_a_ent, chunk_b_ent))
}

pub fn spawn_managed_under_chunk(
	world: &mut World,
	chunk: Entity,
	bounds: TestEntityBounds,
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
