//! Low-poly shoot tube traced along the frond spine.

use bevy::prelude::*;

use super::config::FrondConfig;
use super::spine::{frame_at, spine_at};

const MIN_RADIUS: f32 = 1e-6;

/// Prism tube (`sides` = 4 or 6) following the spine; radius tapers toward the tip.
pub fn append_shoot_tube(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	config: &FrondConfig,
	base_half_radius: f32,
	sides: u32,
) {
	let segments = config.segments.max(1);
	let sides = sides.clamp(3, 8);
	let base_r = base_half_radius.max(MIN_RADIUS);
	let mut rings: Vec<Vec<u32>> = Vec::with_capacity(segments as usize + 1);

	for i in 0..=segments {
		let t = i as f32 / segments as f32;
		let center = spine_at(config, t);
		let (_tangent, lateral, binormal) =
			frame_at(config, t, config.twist * t * std::f32::consts::TAU);
		if !center.is_finite() {
			continue;
		}
		let radius = base_r * (0.35 + 0.65 * (1.0 - t).max(0.0));
		let mut ring = Vec::with_capacity(sides as usize);
		for s in 0..sides {
			let angle = s as f32 / sides as f32 * std::f32::consts::TAU;
			let offset = lateral * angle.cos() + binormal * angle.sin();
			let v = center + offset * radius;
			if !v.is_finite() {
				continue;
			}
			ring.push(positions.len() as u32);
			positions.push(v.to_array());
		}
		if ring.len() as u32 == sides {
			rings.push(ring);
		}
	}

	for window in rings.windows(2) {
		stitch_rings(indices, &window[0], &window[1], sides);
	}
}

fn stitch_rings(indices: &mut Vec<u32>, lower: &[u32], upper: &[u32], sides: u32) {
	for s in 0..sides {
		let s_next = (s + 1) % sides;
		let a = lower[s as usize];
		let b = lower[s_next as usize];
		let c = upper[s as usize];
		let d = upper[s_next as usize];
		indices.extend_from_slice(&[a, c, b, b, c, d]);
	}
}
