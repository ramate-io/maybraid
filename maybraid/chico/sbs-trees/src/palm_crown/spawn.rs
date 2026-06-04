//! Shared stacked [`FrondCrown`](chico_ball_components::frond::FrondCrown) ring spawn for palm trees.

use bevy::prelude::*;
use chico_ball_components::frond::{FrondCrown, FrondCrownShape};
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

/// Per-ring seed salt mixed into foliage noise (shared by Date, Waialea, and Palm Bush).
pub const FROND_RING_SEED_SALT: i32 = 17;

/// Spawn one [`FrondCrown`] mesh per ring at world positions returned by `ring_world_position`.
pub fn spawn_stacked_frond_crowns<LeafM, LeafS>(
	ring_count: u32,
	ring_world_position: impl Fn(u32) -> Vec3,
	frond_shape_for_ring: impl Fn(u32, i32) -> FrondCrownShape,
	frond_world_scale: f32,
	foliage_noise: &NoiseParams,
	leaf_material: LeafS,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	root_transform: Transform,
) -> Vec<Entity>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	let mut out = Vec::new();
	let uniform_scale = frond_world_scale.max(1e-8);

	for ring in 0..ring_count {
		let world_pos = ring_world_position(ring);
		let local = root_transform
			.rotation
			.inverse()
			.mul_vec3(world_pos - root_transform.translation);
		let local_transform =
			Transform { translation: local, scale: Vec3::splat(uniform_scale), ..default() };
		let seed = foliage_noise.seed.wrapping_add(ring as i32 * FROND_RING_SEED_SALT);
		let crown = FrondCrown::from_shape(
			frond_shape_for_ring(ring, seed),
			leaf_material.clone(),
		);
		out.extend(crown.spawn_render_items(commands, cascade_chunk, local_transform));
	}

	out
}
