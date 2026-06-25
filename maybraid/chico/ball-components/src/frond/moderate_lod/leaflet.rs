//! Moderate-LOD frond mesh: shoot tube + densely sampled lateral leaflet cards.

use bevy::prelude::*;

use super::super::config::FrondConfig;
use super::super::shoot::append_shoot_tube;
use super::super::spine::{frame_at, spine_at};

const MIN_HALF: f32 = 1e-6;

/// One leaflet projecting **laterally** from the spine (not along the tangent).
pub fn append_lateral_leaflet(
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
	if !root.is_finite() || !lateral.is_finite() || !binormal.is_finite() {
		return;
	}

	let tip = root + lateral * length;
	let base_lo = root - binormal * half_w;
	let base_hi = root + binormal * half_w;
	let tip_lo = tip - binormal * half_w * 0.25;
	let tip_hi = tip + binormal * half_w * 0.25;

	let base = positions.len() as u32;
	for v in [base_lo, base_hi, tip_lo, tip_hi] {
		if !v.is_finite() {
			return;
		}
		positions.push(v.to_array());
	}
	indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
}

/// Shoot tube plus alternating lateral leaflets along the full spine length.
pub fn append_shoot_and_leaflets(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	config: &FrondConfig,
	shoot_half_radius: f32,
	leaflet_length_scale: f32,
) {
	append_shoot_tube(positions, indices, config, shoot_half_radius, 4);

	let count = config
		.leaflet_count
		.max(2)
		.max(config.segments.saturating_mul(2).saturating_add(1));
	let span = config.length / count as f32;

	for i in 0..count {
		let t = i as f32 / (count - 1) as f32;
		let root = spine_at(config, t);
		let (_tangent, lateral, binormal) =
			frame_at(config, t, config.twist * t * std::f32::consts::TAU);
		let side = if i % 2 == 0 { 1.0 } else { -1.0 };
		let taper = (1.0 - t).max(0.0);
		let half_width = config.width * taper * 0.65;
		let leaflet_length = (config.width * taper * leaflet_length_scale)
			.max(span * 1.1)
			.max(config.width * 0.5 * taper);

		append_lateral_leaflet(
			positions,
			indices,
			root,
			lateral * side,
			binormal,
			half_width,
			leaflet_length,
		);
	}
}
