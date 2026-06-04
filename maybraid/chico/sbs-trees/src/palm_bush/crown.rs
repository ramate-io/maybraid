//! Stacked [`FrondCrown`] rings from a ground anchor (RFC §3.1.7.10).

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_sbs_geometry::PalmBushSbs;
use procedural_common::NoiseParams;
use render_item::CascadeChunk;

use crate::palm_crown::spawn_stacked_frond_crowns;

/// Lower-ring frond length as a fraction of `H` (RFC `0.25`).
const FROND_LENGTH_FRACTION_LO: f32 = 0.25;
/// Upper-ring frond length as a fraction of `H` (RFC `0.40`).
const FROND_LENGTH_FRACTION_HI: f32 = 0.40;
/// Rachis width as a fraction of `H` (RFC `0.05`).
const FROND_WIDTH_FRACTION_OF_HEIGHT: f32 = 0.05;

/// RFC-aligned frond crown defaults scaled to tree height `H`.
pub fn frond_shape_for_ring(
	geometry: &PalmBushSbs,
	ring: u32,
	foliage_seed: i32,
) -> FrondCrownShape {
	let h = geometry.height();
	let scale = geometry.frond_world_scale.max(1e-8);
	let proto = geometry.to_proto();
	let u = proto.ring_vertical_bias(ring);
	let length_fraction = FROND_LENGTH_FRACTION_LO + (FROND_LENGTH_FRACTION_HI - FROND_LENGTH_FRACTION_LO) * u;
	let downward_tilt = 0.38 + (1.0 - u) * 0.18;
	let emission_lift = 0.18 + u * 0.22;
	let droop = 0.52 + (1.0 - u) * 0.16;

	FrondCrownShape {
		frond_count: proto.fronds_per_ring,
		length: (length_fraction * h) / scale,
		width: (FROND_WIDTH_FRACTION_OF_HEIGHT * h) / scale,
		droop,
		arch_lift: 0.22 + u * 0.12,
		twist: 0.16,
		leaflet_count: 14,
		spine_segments: 10,
		shoot_half_radius: 0.014,
		rachis_half_thickness: 0.005,
		leaflet_length_scale: 3.4,
		downward_tilt_radians: downward_tilt,
		outward_spread_radians: 0.95,
		emission_lift_radians: emission_lift,
		seed: foliage_seed.wrapping_add(ring as i32),
	}
}

pub fn spawn_crown_rings<LeafM, LeafS>(
	geometry: &PalmBushSbs,
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
		|ring| geometry.crown_ring_position(ring),
		|ring, seed| frond_shape_for_ring(geometry, ring, seed),
		geometry.frond_world_scale,
		foliage_noise,
		leaf_material,
		commands,
		cascade_chunk,
		root_transform,
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lower_rings_droop_more_than_upper() {
		let geometry = PalmBushSbs::default();
		let low = frond_shape_for_ring(&geometry, 0, 0);
		let high = frond_shape_for_ring(&geometry, geometry.crown.ring_count - 1, 0);
		assert!(low.droop > high.droop);
		assert!(low.downward_tilt_radians > high.downward_tilt_radians);
	}
}
