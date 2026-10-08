//! How each composite development is laid out: fitted to confines on a
//! development cell, or probed over a [`crate::SiteGround`].

mod old_city_market;
mod shepherds_commune;
mod shepherds_village;
mod skybridge_bazaar;
mod suburban_homes;
mod temple_complex;

use bevy_math::bounding::Aabb3d;
use procedural_common::{NoiseParams, SeededHash};

use crate::plan::cell_salt;

/// Root draw for a layout fitted to confines on `cell`.
fn root_hash(cell: Aabb3d, noise: NoiseParams) -> SeededHash {
	SeededHash::new((noise.seed as u32).wrapping_add(cell_salt(cell)))
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t.clamp(0.0, 1.0)
}

/// Lift every site to within `max_relief` of the highest.
fn raise_toward_peak(heights: &mut [Option<f32>], max_relief: f32) {
	let peak = heights.iter().flatten().copied().max_by(f32::total_cmp);
	let Some(peak) = peak else {
		return;
	};
	let floor = peak - max_relief.max(0.0);
	for height in heights.iter_mut().flatten() {
		*height = height.max(floor);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sites_stay_near_the_peak() -> anyhow::Result<()> {
		let mut heights = vec![Some(100.0), Some(72.0), Some(95.0), None];
		raise_toward_peak(&mut heights, 8.0);
		anyhow::ensure!(heights[0] == Some(100.0));
		anyhow::ensure!(heights[1] == Some(92.0));
		anyhow::ensure!(heights[2] == Some(95.0));
		anyhow::ensure!(heights[3].is_none());
		Ok(())
	}
}
