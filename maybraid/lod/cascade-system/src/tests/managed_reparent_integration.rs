//! End-to-end: real [`produce_cascade`](crate::cascade_production::produce_cascade) footprint table
//! plus [`ChunkEntityTrackerPlugin`](crate::chunk_entity_tracker::ChunkEntityTrackerPlugin) for a
//! managed entity under a chunk entity.
//!
//! Uses [`StandardRequirement::signal_on_expired`] = [`RequirementSignal::Hidden`](crate::cascade_production::RequirementSignal::Hidden)
//! so the old leaf chunk stays in [`CascadeProduction::table`](crate::cascade_production::CascadeProduction)
//! while the producer recenters (see RFC-154 §3.4).

use anyhow::Result;
use bevy::prelude::*;

use crate::cascade_production::{CascadeProductionPlugin, RequirementSignal, StandardRequirement};
use crate::chunk_entity_tracker::ChunkEntityTrackerPlugin;
use crate::tests::test_utils::{
	aabb_center_half, chunk_entity_for_footprint, expected_leaf_chunk_for_focal,
	integration_leaf_bounds_recenter, leaf_only_cascade, marked_bounds_at_center_half_extents,
	parent_of_child, spawn_integration_producer, spawn_managed_under_chunk, IntegrationFlow,
	ManagedEntityBounds,
};

#[test]
fn hidden_expiry_keeps_old_chunk_managed_entity_reparents_on_bounds_move() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(CascadeProductionPlugin::<IntegrationFlow>::default());
	app.add_plugins(ChunkEntityTrackerPlugin::<ManagedEntityBounds, IntegrationFlow>::default());

	let cascade = leaf_only_cascade();
	let requirement = StandardRequirement {
		signal_on_new: RequirementSignal::Visible,
		signal_on_expired: RequirementSignal::Hidden,
	};
	let producer = spawn_integration_producer(
		app.world_mut(),
		cascade,
		marked_bounds_at_center_half_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(10.0)),
		requirement,
	);

	app.update();

	let key_a = expected_leaf_chunk_for_focal(Vec3::new(0.5, 0.5, 0.5), &cascade);
	let chunk_a = chunk_entity_for_footprint(app.world(), producer, key_a)?;

	let initial_bounds = ManagedEntityBounds {
		previous: None,
		current: aabb_center_half(Vec3::new(0.5, 0.5, 0.5), 0.05),
	};
	let prev_cur = initial_bounds.current;
	let managed = spawn_managed_under_chunk(app.world_mut(), chunk_a, initial_bounds);

	app.update();
	assert_eq!(parent_of_child(app.world(), managed)?, chunk_a);

	integration_leaf_bounds_recenter(app.world_mut(), producer, Vec3::new(2.5, 0.5, 0.5))?;
	app.update();

	let key_b = expected_leaf_chunk_for_focal(Vec3::new(2.5, 0.5, 0.5), &cascade);
	let chunk_b = chunk_entity_for_footprint(app.world(), producer, key_b)?;

	app.world_mut().entity_mut(managed).insert(ManagedEntityBounds {
		previous: Some(prev_cur),
		current: aabb_center_half(Vec3::new(2.5, 0.5, 0.5), 0.05),
	});

	app.update();
	assert_eq!(parent_of_child(app.world(), managed)?, chunk_b);

	Ok(())
}
