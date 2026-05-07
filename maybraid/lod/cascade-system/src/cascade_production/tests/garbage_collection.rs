//! Requirement signals are wiped at the start of each tick before production runs again.

use bevy::prelude::Vec3;
use lod_cascade::Chunk;

use crate::cascade_production::tests::test_utils::{
	app_with_flow, spawn_orphan_signal, typed_signal_count, AlphaFlow, FlowAlpha,
};
use crate::cascade_production::RequirementSignal;

#[test]
fn garbage_collect_despawns_prior_tick_requirement_signals_before_produce() -> anyhow::Result<()> {
	let mut app = app_with_flow::<FlowAlpha>();
	let world = app.world_mut();
	let chunk = Chunk::from_min_max(Vec3::ZERO, Vec3::ONE, None);

	let signal_entity = spawn_orphan_signal::<FlowAlpha>(world, chunk, RequirementSignal::Hidden);

	assert_eq!(typed_signal_count::<AlphaFlow>(world), 1);
	assert!(world.get_entity(signal_entity).is_ok());

	app.update();

	let world = app.world_mut();
	assert!(
		world.get_entity(signal_entity).is_err(),
		"GC should despawn transient signals before produce runs",
	);
	assert_eq!(typed_signal_count::<AlphaFlow>(world), 0);
	Ok(())
}
