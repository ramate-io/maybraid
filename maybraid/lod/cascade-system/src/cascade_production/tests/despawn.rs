//! Expired footprint handling under configurable [`StandardRequirement`] policies (default **`Remove`**).
//! Also covers **`Hidden`** expiry: entities stay alive with renderer visibility updated.

use bevy::prelude::{Vec3, Visibility};

use super::super::{RequirementSignal, StandardRequirement};
use super::test_utils::{
	app_with_flow, bounds_from_center_half_extents, chunk_entity_alive,
	expected_leaf_chunk_for_focal, leaf_only_cascade, producer_chunk_table_len,
	producer_first_chunk_entity, producer_table_entries, spawn_standard_producer,
	typed_signal_count, AlphaFlow, FlowAlpha,
};

#[test]
fn leaf_recenters_expire_old_chunk_and_swap_table_entity() {
	let mut app = app_with_flow::<FlowAlpha>();
	let cascade = leaf_only_cascade();

	let bounds_a = bounds_from_center_half_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(10.0));
	let producer = spawn_standard_producer::<FlowAlpha>(
		app.world_mut(),
		cascade,
		bounds_a,
		StandardRequirement::default(),
	);

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

#[test]
fn leaf_recenters_expire_hidden_keeps_entity_and_sets_visibility_hidden() {
	let mut app = app_with_flow::<FlowAlpha>();
	let cascade = leaf_only_cascade();

	let requirement = StandardRequirement {
		signal_on_new: RequirementSignal::Visible,
		signal_on_expired: RequirementSignal::Hidden,
	};

	let bounds_a = bounds_from_center_half_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(10.0));
	let producer =
		spawn_standard_producer::<FlowAlpha>(app.world_mut(), cascade, bounds_a, requirement);

	app.update();

	let chunk_entity_first = producer_first_chunk_entity::<FlowAlpha>(app.world(), producer);

	let bounds_b = bounds_from_center_half_extents(Vec3::new(2.5, 0.5, 0.5), Vec3::splat(10.0));
	app.world_mut().entity_mut(producer).insert(bounds_b);

	app.update();

	let world = app.world();
	assert!(
		chunk_entity_alive(world, chunk_entity_first),
		"Hidden expiry keeps the chunk entity alive",
	);
	assert_eq!(world.entity(chunk_entity_first).get::<Visibility>(), Some(&Visibility::Hidden),);

	assert_eq!(
		producer_chunk_table_len::<FlowAlpha>(world, producer),
		2,
		"previous footprint stays in the table while the new leaf is added",
	);

	let world_mut = app.world_mut();
	assert_eq!(
		typed_signal_count::<AlphaFlow>(world_mut),
		1,
		"Hidden expiry emits one transient signal",
	);

	app.update();

	let world_mut = app.world_mut();
	assert_eq!(typed_signal_count::<AlphaFlow>(world_mut), 0);

	let world = app.world();
	let mut found_visible_new_leaf = false;
	for (chunk_key, entity) in producer_table_entries::<FlowAlpha>(world, producer) {
		if entity == chunk_entity_first {
			continue;
		}
		assert_eq!(
			world.entity(entity).get::<Visibility>(),
			Some(&Visibility::Visible),
			"newly entered footprint chunk stays visible",
		);
		let expected = expected_leaf_chunk_for_focal(Vec3::new(2.5, 0.5, 0.5), &cascade);
		assert_eq!(chunk_key, expected);
		found_visible_new_leaf = true;
	}
	assert!(found_visible_new_leaf, "expected a second chunk entity for the new focal cell",);
}
