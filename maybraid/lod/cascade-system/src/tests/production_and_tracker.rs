//! Example: [`CascadeProductionPlugin`] drives footprint work; [`ChunkTrackerPlugin`] reacts to
//! transient [`RequirementSignal`] entities in the same schedule.

use anyhow::Result;
use bevy::prelude::*;

use crate::cascade_production::{CascadeProductionPlugin, RequirementSignal, StandardRequirement};
use crate::chunk_tracker::ChunkTrackerPlugin;
use crate::tests::test_utils::{
	expected_leaf_chunk_for_focal, integration_leaf_bounds_recenter, leaf_only_cascade,
	marked_bounds_at_center_half_extents, observation_count, spawn_integration_producer,
	IntegrationFlow, RecordingChunkTracker, TrackerObservation,
};

#[test]
fn production_and_chunk_tracker_observe_remove_after_recenter() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(CascadeProductionPlugin::<IntegrationFlow>::default());
	app.add_plugins(ChunkTrackerPlugin::<RecordingChunkTracker, IntegrationFlow>::default());

	let cascade = leaf_only_cascade();
	let producer = spawn_integration_producer(
		app.world_mut(),
		cascade,
		marked_bounds_at_center_half_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(10.0)),
		StandardRequirement::default(),
	);

	app.update();
	assert_eq!(observation_count(app.world_mut()), 0);

	integration_leaf_bounds_recenter(app.world_mut(), producer, Vec3::new(2.5, 0.5, 0.5))?;
	app.update();

	assert!(observation_count(app.world_mut()) >= 1, "tracker should record at least one reaction",);
	let mut saw_remove = false;
	{
		let world = app.world_mut();
		for obs in world.query::<&TrackerObservation>().iter(world) {
			if obs.signal == RequirementSignal::Remove {
				saw_remove = true;
			}
		}
	}
	assert!(saw_remove, "expected a Remove signal after leaf recenter");

	let chunk_key = expected_leaf_chunk_for_focal(Vec3::new(0.5, 0.5, 0.5), &cascade);
	let mut saw_removed_footprint = false;
	{
		let world = app.world_mut();
		for obs in world.query::<&TrackerObservation>().iter(world) {
			if obs.chunk == chunk_key && obs.signal == RequirementSignal::Remove {
				saw_removed_footprint = true;
			}
		}
	}
	assert!(saw_removed_footprint, "expected Remove for the leaf cell left behind",);
	Ok(())
}
