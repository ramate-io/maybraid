//! Stochastic apex canopy at the stalk crown (Friend's ball, Temperate fronds).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_sbs_geometry::render::mix_seed::mix_seed_below_fraction;
use chico_sbs_geometry::{
	liams_stalk_tip_from_chain, BallStickChain, BallStickNode, FriendsConiferChain,
	FriendsConiferSbs,
};
use procedural_common::NoiseParams;
use render_item::CascadeChunk;

/// Default fraction of trees that receive an apex canopy cluster (deterministic from [`NoiseParams`] + tip).
pub const DEFAULT_APEX_CANOPY_SPAWN_FRACTION: f32 = 0.72;

/// Friend's apex [`ChicoBall`] world radius as a fraction of stalk height.
pub const FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.024;

/// Northern Conifer apex [`ChicoBall`] world radius as a fraction of stalk height.
pub const NORTHERN_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.022;

/// Gate apex foliage from leaf noise and stalk-tip position.
pub fn sample_apex_canopy_spawn(
	leaf_noise: &NoiseParams,
	tip: &BallStickNode,
	spawn_fraction: f32,
) -> bool {
	let lane = leaf_noise.seed.wrapping_mul(0x9E37) as usize;
	mix_seed_below_fraction(lane, tip.position, spawn_fraction)
}

/// Tree-local transform at a chain tip (chains are generated in tree space; the tree root
/// entity owns world placement).
#[allow(dead_code)]
fn local_transform_at_tip(
	tip: &BallStickNode,
	world_uniform_scale: f32,
	rotation: Quat,
) -> Transform {
	Transform {
		translation: tip.position,
		rotation,
		scale: Vec3::splat(world_uniform_scale.max(1e-8)),
	}
}

/// [`ChicoBall`] at a crown tip (always spawned).
#[allow(dead_code)]
pub fn spawn_apex_chico_ball_at_tip<LeafM, LeafS>(
	tree_height: f32,
	tip: &BallStickNode,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	parent: Entity,
	leaf_noise: &NoiseParams,
	apex_ball_radius_fraction: f32,
	leaf_material: LeafS,
) -> Vec<Entity>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static + Default,
{
	let world_radius = tree_height * apex_ball_radius_fraction;
	spawn_apex_chico_ball_at_tip_with_radius(
		world_radius,
		tip,
		commands,
		cascade_chunk,
		parent,
		leaf_noise,
		leaf_material,
	)
}

/// [`ChicoBall`] at a crown tip with an already-resolved world radius.
#[allow(dead_code)]
pub fn spawn_apex_chico_ball_at_tip_with_radius<LeafM, LeafS>(
	world_radius: f32,
	tip: &BallStickNode,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	parent: Entity,
	leaf_noise: &NoiseParams,
	leaf_material: LeafS,
) -> Vec<Entity>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static + Default,
{
	let apex_noise = leaf_noise.with_seed(leaf_noise.seed.wrapping_add(0xA3E7));
	let mut ball = apex_noise.build_scalar::<ChicoBall<LeafM, LeafS>>();
	ball.material = leaf_material;

	let transform = local_transform_at_tip(tip, world_radius, Quat::IDENTITY);
	ball.spawn_render_items_under(commands, cascade_chunk, transform, Some(parent))
}

/// Optional [`ChicoBall`] at the stalk crown ([#236](https://github.com/ramate-io/maybraid/issues/236)).
#[allow(dead_code)]
pub fn spawn_apex_chico_ball<LeafM, LeafS>(
	geometry: &FriendsConiferSbs,
	chain: &BallStickChain<FriendsConiferChain>,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	parent: Entity,
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
		parent,
		leaf_noise,
		apex_ball_radius_fraction,
		leaf_material,
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use chico_sbs_geometry::FriendsConiferSbs;

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
	fn apex_transform_uses_world_scale_not_tip_radius() {
		let tiny_tip = BallStickNode::new(Vec3::new(0.0, 1.5, 0.0), 0.002);
		let normal_tip = BallStickNode::new(Vec3::new(0.0, 1.5, 0.0), 0.2);
		let tiny = local_transform_at_tip(&tiny_tip, 0.18, Quat::IDENTITY);
		let normal = local_transform_at_tip(&normal_tip, 0.18, Quat::IDENTITY);
		assert!((tiny.scale.x - 0.18).abs() < 1e-5);
		assert_eq!(tiny.scale, normal.scale);
	}

	#[test]
	fn stalk_tip_from_built_friends_chain() {
		let chain = FriendsConiferSbs::default().build_chain();
		let tip = liams_stalk_tip_from_chain(&chain);
		assert!(tip.position.y > 20.0);
	}
}
