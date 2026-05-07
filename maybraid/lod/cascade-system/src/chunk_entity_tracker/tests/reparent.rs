//! Re-parenting when [`TestEntityBounds::current`] moves into another chunk’s footprint.

use bevy::prelude::*;

use crate::chunk_entity_tracker::tests::test_utils::{
	aabb_center_half, adjacent_leaf_chunk_pair, leaf_cascade, parent_of_child,
	spawn_managed_under_chunk, spawn_producer_two_chunks, TestEntityBounds, TestFlow,
};
use crate::chunk_entity_tracker::track_chunk_entities;

#[test]
fn track_chunk_entities_reparents_when_current_bounds_favor_other_chunk() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_systems(Update, track_chunk_entities::<TestEntityBounds, TestFlow>);

	let cascade = leaf_cascade();
	let (chunk_a, chunk_b) = adjacent_leaf_chunk_pair();
	let (_producer, chunk_a_ent, chunk_b_ent) =
		spawn_producer_two_chunks(app.world_mut(), cascade, chunk_a, chunk_b)?;

	let in_cell_a = TestEntityBounds {
		previous: None,
		current: aabb_center_half(Vec3::new(0.5, 0.5, 0.5), 0.05),
	};
	let prev_a = in_cell_a.current;
	let managed = spawn_managed_under_chunk(app.world_mut(), chunk_a_ent, in_cell_a);

	app.update();
	assert_eq!(parent_of_child(app.world(), managed)?, chunk_a_ent);

	let in_cell_b = TestEntityBounds {
		previous: Some(prev_a),
		current: aabb_center_half(Vec3::new(1.5, 0.5, 0.5), 0.05),
	};
	app.world_mut().entity_mut(managed).insert(in_cell_b);

	app.update();
	assert_eq!(parent_of_child(app.world(), managed)?, chunk_b_ent);

	Ok(())
}

#[test]
fn track_chunk_entities_no_op_when_best_chunk_is_current_parent() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_systems(Update, track_chunk_entities::<TestEntityBounds, TestFlow>);

	let cascade = leaf_cascade();
	let (chunk_a, chunk_b) = adjacent_leaf_chunk_pair();
	let (_producer, chunk_a_ent, _chunk_b_ent) =
		spawn_producer_two_chunks(app.world_mut(), cascade, chunk_a, chunk_b)?;

	let bounds = TestEntityBounds {
		previous: None,
		current: aabb_center_half(Vec3::new(0.5, 0.5, 0.5), 0.05),
	};
	let managed = spawn_managed_under_chunk(app.world_mut(), chunk_a_ent, bounds.clone());

	app.update();
	assert_eq!(parent_of_child(app.world(), managed)?, chunk_a_ent);

	app.world_mut().entity_mut(managed).insert(bounds);
	app.update();
	assert_eq!(
		parent_of_child(app.world(), managed)?,
		chunk_a_ent,
		"replacing with identical bounds should not move the entity",
	);

	Ok(())
}
