//! Expired footprints default to [`RequirementSignal::Remove`] under [`StandardRequirement`]:
//! chunk entities despawn and transient Remove signals exist until the next GC pass.

use bevy::prelude::Vec3;

use super::test_utils::{
	app_with_flow, bounds_from_center_half_extents, chunk_entity_alive, leaf_only_cascade,
	producer_chunk_table_len, producer_first_chunk_entity, spawn_standard_producer,
	typed_signal_count, AlphaFlow, FlowAlpha,
};

#[test]
fn leaf_recenters_expire_old_chunk_and_swap_table_entity() {
	let mut app = app_with_flow::<FlowAlpha>();
	let cascade = leaf_only_cascade();

	let bounds_a = bounds_from_center_half_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(10.0));
	let producer = spawn_standard_producer::<FlowAlpha>(app.world_mut(), cascade, bounds_a);

	app.update();

	let chunk_entity_first = producer_first_chunk_entity::<FlowAlpha>(app.world(), producer);
	assert_eq!(producer_chunk_table_len::<FlowAlpha>(app.world(), producer), 1);

	let bounds_b = bounds_from_center_half_extents(Vec3::new(2.5, 0.5, 0.5), Vec3::splat(10.0));
	app.world_mut().entity_mut(producer).insert(bounds_b);

	app.update();

	let world = app.world();
	assert!(
		!chunk_entity_alive(world, chunk_entity_first),
		"expired chunk entity should despawn on Remove",
	);
	assert_eq!(producer_chunk_table_len::<FlowAlpha>(world, producer), 1);

	let chunk_entity_second = producer_first_chunk_entity::<FlowAlpha>(world, producer);
	assert_ne!(chunk_entity_first, chunk_entity_second);

	let world_mut = app.world_mut();
	assert_eq!(
		typed_signal_count::<AlphaFlow>(world_mut),
		1,
		"Remove policy emits one transient footprint signal",
	);

	app.update();

	let world_mut = app.world_mut();
	assert_eq!(
		typed_signal_count::<AlphaFlow>(world_mut),
		0,
		"second tick GC clears Remove signals",
	);
	let world = app.world();
	assert!(chunk_entity_alive(world, chunk_entity_second));
}
