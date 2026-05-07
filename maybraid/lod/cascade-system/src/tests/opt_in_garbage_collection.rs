//! **Opt-in garbage collection**: [`CascadeProductionPlugin`] always runs
//! **`garbage_collect_requirement_signals`** *before* **`produce_cascade`**. When you need trackers
//! (or other systems) to observe signals *before* they are cleared, compose your own schedule.
//!
//! This module shows a **manual** `produce → track → GC` chain with GC gated so the first tick keeps
//! transient entities alive for inspection, then a later tick clears them.

use anyhow::Result;
use bevy::prelude::*;

use crate::cascade_production::{
	garbage_collect_requirement_signals, produce_cascade, StandardRequirement,
};
use crate::chunk_tracker::track_chunks;
use crate::tests::test_utils::{
	gc_run_condition, integration_leaf_bounds_recenter, leaf_only_cascade,
	marked_bounds_at_center_half_extents, observation_count, requirement_signal_entity_count,
	spawn_integration_producer, GcCounter, IntegrationFlow, RecordingChunkTracker,
};

#[test]
fn manual_produce_track_then_gc_allows_tracker_before_cleanup() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.insert_resource(GcCounter(0));

	app.add_systems(Update, produce_cascade::<IntegrationFlow>);
	app.add_systems(
		Update,
		track_chunks::<RecordingChunkTracker, IntegrationFlow>
			.after(produce_cascade::<IntegrationFlow>),
	);
	app.add_systems(
		Update,
		garbage_collect_requirement_signals::<IntegrationFlow>
			.after(track_chunks::<RecordingChunkTracker, IntegrationFlow>)
			.run_if(gc_run_condition),
	);

	let cascade = leaf_only_cascade();
	let producer = spawn_integration_producer(
		app.world_mut(),
		cascade,
		marked_bounds_at_center_half_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(10.0)),
		StandardRequirement::default(),
	);

	app.update();

	integration_leaf_bounds_recenter(app.world_mut(), producer, Vec3::new(2.5, 0.5, 0.5))?;
	app.update();

	assert!(
		observation_count(app.world_mut()) >= 1,
		"tracker should run before opt-in GC on the second tick",
	);
	assert!(
		requirement_signal_entity_count::<IntegrationFlow>(app.world_mut()) >= 1,
		"signals should still exist while GcCounter is 0 (GC skipped)",
	);

	app.world_mut().resource_mut::<GcCounter>().0 = 1;

	app.update();

	assert_eq!(
		requirement_signal_entity_count::<IntegrationFlow>(app.world_mut()),
		0,
		"after GC runs (GcCounter > 0), transient signals should be gone",
	);

	Ok(())
}
