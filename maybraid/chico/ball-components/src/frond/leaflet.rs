//! Leaflet quads along a frond spine.

use bevy::prelude::*;

use super::config::FrondConfig;
use super::spine::{frame_at, spine_at};

/// Emit one tapered leaflet as two triangles (single-sided; use double-sided foliage material).
pub fn append_leaflet(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	root: Vec3,
	tangent: Vec3,
	lateral: Vec3,
	half_width: f32,
	leaflet_length: f32,
) {
	if half_width < 1e-8 || leaflet_length < 1e-8 {
		return;
	}
	if !root.is_finite() || !tangent.is_finite() || !lateral.is_finite() {
		return;
	}

	let tip = root + tangent * leaflet_length;
	let left = root - lateral * half_width;
	let right = root + lateral * half_width;
	let tip_left = tip - lateral * half_width * 0.35;
	let tip_right = tip + lateral * half_width * 0.35;

	let base = positions.len() as u32;
	for v in [left, right, tip_left, tip_right] {
		if !v.is_finite() {
			return;
		}
		positions.push(v.to_array());
	}
	indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
}

/// Place alternating leaflets along the spine per RFC §3.1.2.7.
pub fn append_leaflets_along_spine(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	config: &FrondConfig,
) {
	let count = config.leaflet_count.max(2);
	let span = config.length / count as f32;

	for i in 0..count {
		let t = i as f32 / (count - 1) as f32;
		let root = spine_at(config, t);
		let (tangent, lateral, _binormal) =
			frame_at(config, t, config.twist * t * std::f32::consts::TAU);
		let side = if i % 2 == 0 { 1.0 } else { -1.0 };
		let half_width = config.width * (1.0 - t) * 0.5;
		let leaflet_length = span * 1.15;

		append_leaflet(
			positions,
			indices,
			root,
			tangent,
			lateral * side,
			half_width,
			leaflet_length,
		);
	}
}
