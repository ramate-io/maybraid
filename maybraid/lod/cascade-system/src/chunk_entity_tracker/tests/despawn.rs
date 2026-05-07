//! Despawn when chunk selection finds no table entity.

use anyhow::{anyhow, Result};
use bevy::prelude::*;

use crate::cascade_production::{CascadeProduction, CascadeTable};
use crate::chunk_entity_tracker::tests::test_utils::{
	aabb_center_half, adjacent_leaf_chunk_pair, leaf_cascade, spawn_managed_under_chunk,
	spawn_producer_two_chunks, TestEntityBounds, TestFlow,
};
use crate::chunk_entity_tracker::track_chunk_entities;

#[test]
fn track_chunk_entities_despawns_when_select_chunk_finds_nothing() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_systems(Update, track_chunk_entities::<TestEntityBounds, TestFlow>);

	let cascade = leaf_cascade();
	let (chunk_a, chunk_b) = adjacent_leaf_chunk_pair();
	let (producer, chunk_a_ent, _chunk_b_ent) =
		spawn_producer_two_chunks(app.world_mut(), cascade, chunk_a, chunk_b)?;

	{
		let world = app.world_mut();
		let mut entity = world.entity_mut(producer);
		let mut prod = entity
			.get_mut::<CascadeProduction<TestFlow>>()
			.ok_or_else(|| anyhow!("producer missing CascadeProduction"))?;
		prod.table = CascadeTable::default();
	}

	let bounds = TestEntityBounds {
		previous: None,
		current: aabb_center_half(Vec3::new(0.5, 0.5, 0.5), 0.05),
	};
	let managed = spawn_managed_under_chunk(app.world_mut(), chunk_a_ent, bounds);

	app.update();
	assert!(app.world().get_entity(managed).is_err());

	Ok(())
}
