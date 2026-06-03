//! Stochastic apex canopy at the stalk crown (Friend's ball, Temperate fronds).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::frond::{align_frond_direction, FrondCrown, FrondCrownShape};
use chico_sbs_geometry::render::mix_seed::mix_seed_below_fraction;
use chico_sbs_geometry::{
	liams_stalk_tip_from_chain, BallStickChain, BallStickNode, FriendsConiferChain, FriendsConiferSbs,
};
use procedural_common::NoiseParams;
use render_item::CascadeChunk;

/// Default fraction of trees that receive an apex canopy cluster (deterministic from [`NoiseParams`] + tip).
pub const DEFAULT_APEX_CANOPY_SPAWN_FRACTION: f32 = 0.72;

/// Friend's apex [`ChicoBall`] world radius as a fraction of stalk height.
pub const FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.024;

/// Gate apex foliage from leaf noise and stalk-tip position.
pub fn sample_apex_canopy_spawn(
	leaf_noise: &NoiseParams,
	tip: &BallStickNode,
	spawn_fraction: f32,
) -> bool {
	let lane = leaf_noise.seed.wrapping_mul(0x9E37) as usize;
	mix_seed_below_fraction(lane, tip.position, spawn_fraction)
}

fn local_transform_at_tip(
	root_transform: Transform,
	tip: &BallStickNode,
	world_uniform_scale: f32,
	rotation: Quat,
) -> Transform {
	let local = root_transform
		.rotation
		.inverse()
		.mul_vec3(tip.position - root_transform.translation);
	Transform {
		translation: local,
		rotation,
		scale: Vec3::splat(world_uniform_scale / tip.radius.max(1e-4)),
		..default()
	}
}

/// [`ChicoBall`] at a crown tip (always spawned).
pub fn spawn_apex_chico_ball_at_tip<LeafM, LeafS>(
	tree_height: f32,
	tip: &BallStickNode,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	root_transform: Transform,
	leaf_noise: &NoiseParams,
	apex_ball_radius_fraction: f32,
	leaf_material: LeafS,
) -> Vec<Entity>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static + Default,
{
	let world_radius = tree_height * apex_ball_radius_fraction;
	let apex_noise = leaf_noise.with_seed(leaf_noise.seed.wrapping_add(0xA3E7));
	let mut ball = apex_noise.build_scalar::<ChicoBall<LeafM, LeafS>>();
	ball.material = leaf_material;

	let transform = local_transform_at_tip(root_transform, tip, world_radius, Quat::IDENTITY);
	ball.spawn_render_items_under(commands, cascade_chunk, transform, None)
}

/// Optional [`ChicoBall`] at the stalk crown ([#236](https://github.com/ramate-io/maybraid/issues/236)).
pub fn spawn_apex_chico_ball<LeafM, LeafS>(
	geometry: &FriendsConiferSbs,
	chain: &BallStickChain<FriendsConiferChain>,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	root_transform: Transform,
	leaf_noise: &NoiseParams,
	apex_spawn_fraction: f32,
	apex_ball_radius_fraction: f32,
	leaf_material: LeafS,
) -> Vec<Entity>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static + Default,
{
	let tip = liams_stalk_tip_from_chain(chain);
	if !sample_apex_canopy_spawn(leaf_noise, &tip, apex_spawn_fraction) {
		return Vec::new();
	}

	spawn_apex_chico_ball_at_tip(
		geometry.height(),
		&tip,
		commands,
		cascade_chunk,
		root_transform,
		leaf_noise,
		apex_ball_radius_fraction,
		leaf_material,
	)
}

/// Optional downward [`FrondCrown`] at the stalk crown ([#238](https://github.com/ramate-io/maybraid/issues/238)).
pub fn spawn_apex_frond_crown<LeafM, LeafS>(
	geometry: &FriendsConiferSbs,
	frond_world_scale: f32,
	chain: &BallStickChain<FriendsConiferChain>,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	root_transform: Transform,
	leaf_noise: &NoiseParams,
	apex_spawn_fraction: f32,
	leaf_material: LeafS,
) -> Vec<Entity>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	let tip = liams_stalk_tip_from_chain(chain);
	if !sample_apex_canopy_spawn(leaf_noise, &tip, apex_spawn_fraction) {
		return Vec::new();
	}

	let h = geometry.height();
	let scale = frond_world_scale.max(1e-8);
	let seed = leaf_noise.seed.wrapping_add(0xC1A0);
	let frond_count = 4 + ((seed as u32) % 3);

	let shape = FrondCrownShape {
		frond_count,
		length: (0.065 * h) / scale,
		width: (0.014 * h) / scale,
		droop: 0.32,
		arch_lift: 0.08,
		twist: 0.15,
		leaflet_count: 10,
		spine_segments: 8,
		shoot_half_radius: 0.008,
		rachis_half_thickness: 0.004,
		leaflet_length_scale: 2.8,
		downward_tilt_radians: 0.42,
		outward_spread_radians: 0.55,
		emission_lift_radians: 0.05,
		seed,
	};

	let crown = FrondCrown::from_shape(shape, leaf_material);
	let transform = local_transform_at_tip(
		root_transform,
		&tip,
		frond_world_scale,
		align_frond_direction(Vec3::NEG_Y),
	);
	crown.spawn_render_items_under(commands, cascade_chunk, transform, None)
}

#[cfg(test)]
mod tests {
	use super::*;
	use chico_sbs_geometry::FriendsConiferSbs;
	use procedural_common::FromScalarNoise;

	#[test]
	fn apex_gate_is_deterministic() {
		let noise = NoiseParams::from_scalar(42.0, 1.0, 1.0, 1);
		let tip = BallStickNode::new(Vec3::new(0.0, 30.0, 0.0), 0.2);
		let a = sample_apex_canopy_spawn(&noise, &tip, 0.72);
		let b = sample_apex_canopy_spawn(&noise, &tip, 0.72);
		assert_eq!(a, b);
	}

	#[test]
	fn apex_gate_respects_fraction_bounds() {
		let noise = NoiseParams::from_scalar(7.0, 1.0, 1.0, 1);
		let tip = BallStickNode::new(Vec3::new(1.0, 25.0, 2.0), 0.15);
		assert!(!sample_apex_canopy_spawn(&noise, &tip, 0.0));
		assert!(sample_apex_canopy_spawn(&noise, &tip, 1.0));
	}

	#[test]
	fn stalk_tip_from_built_friends_chain() {
		let chain = FriendsConiferSbs::default().build_chain();
		let tip = liams_stalk_tip_from_chain(&chain);
		assert!(tip.position.y > 20.0);
	}
}
