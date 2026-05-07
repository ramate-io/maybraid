//! First-tick chunk creation and stability across ticks when bounds are unchanged.

use bevy::prelude::*;
use lod_cascade::Chunk;

use super::super::CascadeChunk;
use super::test_utils::{
	app_with_flow, bounds_from_center_half_extents, chunk_entity_alive, leaf_only_cascade,
	producer_chunk_table_len, producer_first_chunk_entity, spawn_standard_producer, FlowAlpha,
};

#[test]
fn leaf_only_cascade_spawns_single_chunk_and_parents_under_producer() {
	let mut app = app_with_flow::<FlowAlpha>();
	let cascade = leaf_only_cascade();
	let bounds = bounds_from_center_half_extents(Vec3::splat(0.5), Vec3::splat(50.0));

	let producer = spawn_standard_producer::<FlowAlpha>(app.world_mut(), cascade, bounds);

	app.update();

	let world = app.world();
	assert_eq!(producer_chunk_table_len::<FlowAlpha>(world, producer), 1);

	let chunk_entity = producer_first_chunk_entity::<FlowAlpha>(world, producer);
	assert!(chunk_entity_alive(world, chunk_entity));
	assert!(
		world.entity(chunk_entity).get::<CascadeChunk>().is_some(),
		"spawned chunk carries CascadeChunk",
	);

	let expected_chunk = Chunk::from_min_max(Vec3::ZERO, Vec3::ONE, None);
	assert_eq!(world.entity(chunk_entity).get::<CascadeChunk>().unwrap().0, expected_chunk);

	let children = world
		.entity(producer)
		.get::<Children>()
		.expect("producer should own chunk children");
	assert!(
		children.iter().any(|c| c == chunk_entity),
		"chunk entity should be a child of the producer",
	);
}

#[test]
fn unchanged_bounds_second_tick_keeps_same_chunk_entity() {
	let mut app = app_with_flow::<FlowAlpha>();
	let cascade = leaf_only_cascade();
	let bounds = bounds_from_center_half_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(10.0));

	let producer = spawn_standard_producer::<FlowAlpha>(app.world_mut(), cascade, bounds);

	app.update();
	let chunk_after_first = producer_first_chunk_entity::<FlowAlpha>(app.world(), producer);

	app.update();
	let chunk_after_second = producer_first_chunk_entity::<FlowAlpha>(app.world(), producer);

	assert_eq!(chunk_after_first, chunk_after_second);
	assert_eq!(producer_chunk_table_len::<FlowAlpha>(app.world(), producer), 1);
}
