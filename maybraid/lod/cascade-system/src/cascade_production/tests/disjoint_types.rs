//! Different [`StandardFlow`] instantiations use disjoint marker components so garbage collection and
//! production queries do not cross wires.

use bevy::prelude::Vec3;
use lod_cascade::Chunk;

use super::super::{RequirementSignal, StandardRequirement};
use super::test_utils::{
	app_alpha_only, app_dual_flow, chunk_footprint, expected_leaf_chunk_for_focal,
	leaf_only_cascade, marked_bounds_at_center_half_extents, producer_children,
	producer_chunk_table_len, producer_first_chunk_entity, spawn_orphan_signal,
	spawn_standard_producer, typed_signal_count, AlphaFlow, BetaFlow, FlowAlpha, FlowBeta,
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

#[test]
fn dual_flows_spawn_distinct_leaf_chunks_matching_geometry() {
	let mut app = app_dual_flow();
	let cascade = leaf_only_cascade();

	let alpha_center = Vec3::new(0.5, 0.5, 0.5);
	let beta_center = Vec3::new(10.5, 0.5, 0.5);

	let alpha_bounds =
		marked_bounds_at_center_half_extents::<FlowAlpha>(alpha_center, Vec3::splat(10.0));
	let beta_bounds =
		marked_bounds_at_center_half_extents::<FlowBeta>(beta_center, Vec3::splat(10.0));

	let alpha_prod = spawn_standard_producer::<FlowAlpha>(
		app.world_mut(),
		cascade,
		alpha_bounds,
		StandardRequirement::default(),
	);
	let beta_prod = spawn_standard_producer::<FlowBeta>(
		app.world_mut(),
		cascade,
		beta_bounds,
		StandardRequirement::default(),
	);

	app.update();

	let world = app.world();
	assert_eq!(producer_chunk_table_len::<FlowAlpha>(world, alpha_prod), 1);
	assert_eq!(producer_chunk_table_len::<FlowBeta>(world, beta_prod), 1);

	let expected_alpha = expected_leaf_chunk_for_focal(alpha_center, &cascade);
	let expected_beta = expected_leaf_chunk_for_focal(beta_center, &cascade);
	assert_ne!(
		expected_alpha, expected_beta,
		"fixture focal cells should disagree so flows stay distinguishable",
	);

	let alpha_work = cascade.cascade_footprints(alpha_center);
	let beta_work = cascade.cascade_footprints(beta_center);
	assert!(
		alpha_work.contains(&expected_alpha),
		"alpha expected chunk must appear in cascade footprint set at alpha focal",
	);
	assert!(
		beta_work.contains(&expected_beta),
		"beta expected chunk must appear in cascade footprint set at beta focal",
	);

	let alpha_chunk_ent = producer_first_chunk_entity::<FlowAlpha>(world, alpha_prod);
	let beta_chunk_ent = producer_first_chunk_entity::<FlowBeta>(world, beta_prod);

	assert_ne!(alpha_chunk_ent, beta_chunk_ent);

	let alpha_fp = chunk_footprint(world, alpha_chunk_ent);
	let beta_fp = chunk_footprint(world, beta_chunk_ent);

	assert_eq!(alpha_fp, expected_alpha);
	assert_eq!(beta_fp, expected_beta);
	assert_eq!(alpha_fp.extent(), cascade.leaf_scale());
	assert_eq!(beta_fp.extent(), cascade.leaf_scale());

	let alpha_children = producer_children(world, alpha_prod);
	let beta_children = producer_children(world, beta_prod);

	assert!(alpha_children.iter().any(|c| *c == alpha_chunk_ent));
	assert!(beta_children.iter().any(|c| *c == beta_chunk_ent));
}
