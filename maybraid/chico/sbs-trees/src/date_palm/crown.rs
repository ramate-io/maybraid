//! Stacked [`FrondCrown`] rings at the trunk tip (RFC palm crown).

use bevy::prelude::*;
use chico_ball_components::frond::{FrondCrown, FrondCrownShape};
use chico_sbs_geometry::{BallStickChain, DatePalmChain, DatePalmSbs};
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

/// Frond spine length in world units (`shape.length * frond_world_scale`).
const FROND_LENGTH_FRACTION_OF_HEIGHT: f32 = 0.44;
/// Rachis width in world units before crown uniform scale.
const FROND_WIDTH_FRACTION_OF_HEIGHT: f32 = 0.058;

/// RFC-aligned frond crown defaults scaled to tree height `H`.
pub fn frond_shape_for_ring(geometry: &DatePalmSbs, ring: u32, foliage_seed: i32) -> FrondCrownShape {
	let h = geometry.height();
	let scale = geometry.frond_world_scale.max(1e-8);
	let proto = geometry.to_proto();
	let downward_tilt = 0.30 + proto.ring_vertical_bias(ring) * 0.26;

	FrondCrownShape {
		frond_count: proto.fronds_per_ring,
		length: (FROND_LENGTH_FRACTION_OF_HEIGHT * h) / scale,
		width: (FROND_WIDTH_FRACTION_OF_HEIGHT * h) / scale,
		droop: 0.36,
		twist: 0.21,
		leaflet_count: 16,
		spine_segments: 11,
		shoot_half_radius: 0.018,
		rachis_half_thickness: 0.007,
		leaflet_length_scale: 2.5,
		downward_tilt_radians: downward_tilt,
		outward_spread_radians: 0.78,
		seed: foliage_seed.wrapping_add(ring as i32),
	}
}

pub fn spawn_crown_rings<LeafM, LeafS>(
	geometry: &DatePalmSbs,
	chain: &BallStickChain<DatePalmChain>,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	root_transform: Transform,
	foliage_noise: &NoiseParams,
	leaf_material: LeafS,
) -> Vec<Entity>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	let mut out = Vec::new();
	let ring_count = geometry.crown.ring_count;
	let uniform_scale = geometry.frond_world_scale;

	for ring in 0..ring_count {
		let world_pos = geometry.crown_ring_position(chain, ring);
		let local = root_transform
			.rotation
			.inverse()
			.mul_vec3(world_pos - root_transform.translation);
		let local_transform = Transform {
			translation: local,
			scale: Vec3::splat(uniform_scale),
			..default()
		};
		let seed = foliage_noise.seed.wrapping_add(ring as i32 * 17);
		let crown = FrondCrown::from_shape(
			frond_shape_for_ring(geometry, ring, seed),
			leaf_material.clone(),
		);
		out.extend(crown.spawn_render_items(commands, cascade_chunk, local_transform));
	}

	out
}
