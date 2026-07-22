//! Stream path placement and graded water / bank / bed levels.

use crate::noise::n01_at;
use bevy_math::Vec2;

pub(crate) const ENDPOINT_A_SALT: u32 = 0x57EA_E001;
pub(crate) const ENDPOINT_B_SALT: u32 = 0x57EA_E002;

pub(crate) fn sample_endpoint(seed: u32, salt: u32, lo: Vec2, hi: Vec2) -> Vec2 {
	let ux = n01_at(seed, salt, lo);
	let uz = n01_at(seed, salt.wrapping_add(1), lo);
	Vec2::new(
		lo.x + (hi.x - lo.x) * ux,
		lo.y + (hi.y - lo.y) * uz,
	)
}

/// Sample per-node water elevations along a path (pre-watershed heights − sink).
///
/// Local segment pitches follow the samples; uphill chords are clamped so each
/// node is non-increasing vs its upstream neighbor. If the whole reach is nearly
/// flat, the toe is pulled down by `min_drop`.
pub(crate) fn node_water_levels(
	path: &[Vec2],
	height_at: Option<&dyn Fn(f32, f32) -> f32>,
	sink: f32,
	min_drop: f32,
) -> Vec<f32> {
	let sink = sink.max(0.0);
	let min_drop = min_drop.max(0.0);
	let mut levels: Vec<f32> = path
		.iter()
		.map(|p| height_at.map(|f| f(p.x, p.y)).unwrap_or(0.0) - sink)
		.collect();
	for i in 1..levels.len() {
		levels[i] = levels[i].min(levels[i - 1]);
	}
	if levels.len() >= 2 {
		let head = levels[0];
		let last = levels.len() - 1;
		if head - levels[last] < min_drop {
			levels[last] = head - min_drop;
			for i in (1..last).rev() {
				levels[i] = levels[i]
					.min(levels[i - 1])
					.max(levels[last]);
			}
		}
	}
	levels
}

pub(crate) fn bank_levels(water_levels: &[f32], rim_lift: f32) -> Vec<f32> {
	let lift = rim_lift.max(0.0);
	water_levels.iter().map(|w| w + lift).collect()
}

/// Channel floor grade: water surface levels minus freeboard (strictly below \(W\)).
pub(crate) fn bed_levels(water_levels: &[f32], freeboard: f32) -> Vec<f32> {
	let fb = freeboard.max(0.25);
	water_levels.iter().map(|w| w - fb).collect()
}
