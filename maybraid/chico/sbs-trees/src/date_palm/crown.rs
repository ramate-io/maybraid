//! Stacked [`FrondCrown`] rings at the trunk tip (RFC palm crown).

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_sbs_geometry::{BallStickChain, DatePalmChain, DatePalmSbs};
use procedural_common::NoiseParams;
use render_item::CascadeChunk;

use crate::palm_crown::spawn_stacked_frond_crowns;

/// Frond spine length in world units (`shape.length * frond_world_scale`).
const FROND_LENGTH_FRACTION_OF_HEIGHT: f32 = 0.7;
/// Rachis width in world units before crown uniform scale.
const FROND_WIDTH_FRACTION_OF_HEIGHT: f32 = 0.070;

/// RFC-aligned frond crown defaults scaled to tree height `H`.
pub fn frond_shape_for_ring(
	geometry: &DatePalmSbs,
	ring: u32,
	foliage_seed: i32,
) -> FrondCrownShape {
	let h = geometry.height();
	let scale = geometry.frond_world_scale.max(1e-8);
	let proto = geometry.to_proto();
	let u = proto.ring_vertical_bias(ring);
	let downward_tilt = 0.22 + u * 0.20;
	let emission_lift = 0.28 + (1.0 - u) * 0.10;

	FrondCrownShape {
		frond_count: proto.fronds_per_ring,
		length: (FROND_LENGTH_FRACTION_OF_HEIGHT * h) / scale,
		width: (FROND_WIDTH_FRACTION_OF_HEIGHT * h) / scale,
		droop: 0.44,
		arch_lift: 0.30,
		twist: 0.21,
		leaflet_count: 16,
		spine_segments: 11,
		shoot_half_radius: 0.020,
		rachis_half_thickness: 0.007,
		leaflet_length_scale: 4.0,
		downward_tilt_radians: downward_tilt,
		outward_spread_radians: 1.0,
		emission_lift_radians: emission_lift,
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
	spawn_stacked_frond_crowns::<LeafM, LeafS>(
		geometry.crown.ring_count,
		|ring| geometry.crown_ring_position(chain, ring),
		|ring, seed| frond_shape_for_ring(geometry, ring, seed),
		geometry.frond_world_scale,
		foliage_noise.seed,
		leaf_material,
		commands,
		cascade_chunk,
		root_transform,
	)
}
