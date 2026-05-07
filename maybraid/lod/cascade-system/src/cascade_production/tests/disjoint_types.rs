//! Different [`StandardFlow`] instantiations use disjoint marker components so GC / queries do not cross wires.

use bevy::prelude::Vec3;
use lod_cascade::Chunk;

use super::super::RequirementSignal;
use super::test_utils::{
	app_alpha_only, app_dual_flow, spawn_orphan_signal, typed_signal_count, AlphaFlow, BetaFlow,
	FlowAlpha, FlowBeta,
};

#[test]
fn beta_marked_signal_survives_when_only_alpha_plugin_registered() {
	let mut app = app_alpha_only();
	let chunk = Chunk::from_min_max(Vec3::ZERO, Vec3::ONE, None);

	let beta_signal =
		spawn_orphan_signal::<FlowBeta>(app.world_mut(), chunk, RequirementSignal::Hidden);

	app.update();

	let world = app.world_mut();
	assert!(
		world.get_entity(beta_signal).is_ok(),
		"without BetaFlow plugin there is no GC system for BetaFlow markers",
	);
	assert_eq!(typed_signal_count::<BetaFlow>(world), 1);
	assert_eq!(typed_signal_count::<AlphaFlow>(world), 0);
}

#[test]
fn dual_plugins_collect_each_flow_independently() {
	let mut app = app_dual_flow();
	let chunk = Chunk::from_min_max(Vec3::ZERO, Vec3::ONE, None);

	let alpha_entity =
		spawn_orphan_signal::<FlowAlpha>(app.world_mut(), chunk, RequirementSignal::Hidden);
	let beta_entity =
		spawn_orphan_signal::<FlowBeta>(app.world_mut(), chunk, RequirementSignal::Hidden);

	app.update();

	let world = app.world_mut();
	assert!(world.get_entity(alpha_entity).is_err());
	assert!(world.get_entity(beta_entity).is_err());
	assert_eq!(typed_signal_count::<AlphaFlow>(world), 0);
	assert_eq!(typed_signal_count::<BetaFlow>(world), 0);
}
