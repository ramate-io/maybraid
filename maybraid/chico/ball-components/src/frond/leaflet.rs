//! High-LOD frond mesh: shoot tube + densely sampled lateral leaflet pairs.

use bevy::prelude::*;

use super::config::FrondConfig;
use super::shoot::append_shoot_tube;
use super::spine::{frame_at, spine_at};

const MIN_HALF: f32 = 1e-6;

/// Leaflet pair projecting laterally from a spine sample (± lateral).
pub fn append_lateral_leaflet_pair(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	root: Vec3,
	lateral: Vec3,
	binormal: Vec3,
	half_width: f32,
	leaflet_length: f32,
) {
	let half_w = half_width.max(MIN_HALF);
	let length = leaflet_length.max(MIN_HALF);
	for side in [-1.0_f32, 1.0] {
		let lat = lateral * side;
		let tip = root + lat * length;
		let lo = root - binormal * half_w;
		let hi = root + binormal * half_w;
		let tip_lo = tip - binormal * half_w * 0.2;
		let tip_hi = tip + binormal * half_w * 0.2;
		let base = positions.len() as u32;
		for v in [lo, hi, tip_lo, tip_hi] {
			if !v.is_finite() {
				return;
			}
			positions.push(v.to_array());
		}
		indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
	}
}

/// Shoot tube plus leaflet pairs at evenly spaced heights along the spine.
pub fn append_spine_shoot_and_leaflets(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	config: &FrondConfig,
	shoot_half_radius: f32,
	shoot_sides: u32,
	leaflet_length_scale: f32,
) {
	append_shoot_tube(positions, indices, config, shoot_half_radius, shoot_sides);

	let count = config.leaflet_count.max(2);
	let span = config.length / count as f32;

	for i in 0..count {
		let t = i as f32 / (count - 1) as f32;
		let center = spine_at(config, t);
		let (_tangent, lateral, binormal) =
			frame_at(config, t, config.twist * t * std::f32::consts::TAU);
		if !center.is_finite() {
			continue;
		}

		let taper = (1.0 - t).max(0.0);
		let half_width = config.width * taper * 0.55;
		let leaflet_length = (config.width * taper * leaflet_length_scale)
			.max(span * 1.05)
			.max(config.width * 0.45 * taper);

		append_lateral_leaflet_pair(
			positions,
			indices,
			center,
			lateral,
			binormal,
			half_width,
			leaflet_length,
		);
	}
}
