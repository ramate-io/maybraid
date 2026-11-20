//! CPU mirror of `lod.wesl`.

use crate::wesl::layout::{GRASS_LOD_CARD, GRASS_LOD_MID, GRASS_LOD_NEAR};

/// Distance (metres) and density (`0..=1`) thresholds. Nested: card ⊂ mid ⊂ near.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodThresholds {
	pub near_m: f32,
	pub mid_m: f32,
	pub card_m: f32,
	pub near_density: f32,
	pub mid_density: f32,
	pub card_density: f32,
	pub min_height_px: f32,
}

impl LodThresholds {
	pub fn field_default() -> Self {
		Self {
			near_m: 18.0,
			mid_m: 48.0,
			card_m: 90.0,
			near_density: 1.0,
			mid_density: 0.72,
			card_density: 0.38,
			min_height_px: 1.15,
		}
	}

	pub fn nested_ok(self) -> bool {
		self.near_density >= self.mid_density && self.mid_density >= self.card_density
	}
}

/// Projected plant height at the actual render resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedSize {
	pub height_px: f32,
}

impl ProjectedSize {
	pub fn height_px(
		height_m: f32,
		distance_m: f32,
		view_height_px: f32,
		fov_y_radians: f32,
	) -> Self {
		Self { height_px: projected_height_px(height_m, distance_m, view_height_px, fov_y_radians) }
	}
}

/// Selected geometry bucket plus whether the plant is submitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LodDecision {
	pub bucket: u32,
}

impl LodDecision {
	pub fn is_submitted(self) -> bool {
		self.bucket < 3
	}
}

pub fn projected_height_px(
	height_m: f32,
	distance_m: f32,
	view_height_px: f32,
	fov_y_radians: f32,
) -> f32 {
	let dist = distance_m.max(1e-3);
	let denom = 2.0 * (fov_y_radians * 0.5).tan();
	view_height_px * (height_m / dist) / denom.max(1e-5)
}

pub fn lod_fade(distance_m: f32, start_m: f32, end_m: f32) -> f32 {
	((end_m - distance_m) / (end_m - start_m).max(1e-3)).clamp(0.0, 1.0)
}

pub fn nested_keep(rank: f32, density: f32) -> bool {
	rank < density.clamp(0.0, 1.0)
}

pub fn lod_bucket(
	distance_m: f32,
	height_m: f32,
	rank: f32,
	view_height_px: f32,
	fov_y_radians: f32,
	thresholds: LodThresholds,
) -> u32 {
	let px = projected_height_px(height_m, distance_m, view_height_px, fov_y_radians);
	if px < thresholds.min_height_px && distance_m > thresholds.card_m {
		return 3;
	}
	if distance_m < thresholds.near_m && nested_keep(rank, thresholds.near_density) {
		return GRASS_LOD_NEAR;
	}
	if distance_m < thresholds.mid_m && nested_keep(rank, thresholds.mid_density) {
		return GRASS_LOD_MID;
	}
	if distance_m < thresholds.card_m && nested_keep(rank, thresholds.card_density) {
		return GRASS_LOD_CARD;
	}
	3
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::wesl::hash::VegetationHash;

	#[test]
	fn nested_density_is_stable_across_distance() -> anyhow::Result<()> {
		let t = LodThresholds::field_default();
		assert!(t.nested_ok());
		let mut far_ids = Vec::new();
		for i in 0..64u32 {
			let rank = VegetationHash::density_rank(VegetationHash::plant_id(11, i));
			let far = lod_bucket(70.0, 0.55, rank, 720.0, 1.0, t);
			if far == GRASS_LOD_CARD {
				far_ids.push(i);
				let mid = lod_bucket(30.0, 0.55, rank, 720.0, 1.0, t);
				assert!(mid == GRASS_LOD_MID || mid == GRASS_LOD_NEAR);
			}
		}
		assert!(!far_ids.is_empty());
		Ok(())
	}

	#[test]
	fn skyline_height_is_not_dropped_for_subpixel_width_alone() -> anyhow::Result<()> {
		let t = LodThresholds::field_default();
		// Distant but still tall in pixels (crest) stays at least a card.
		let bucket = lod_bucket(80.0, 0.7, 0.01, 1080.0, 1.0, t);
		assert_eq!(bucket, GRASS_LOD_CARD);
		Ok(())
	}

	#[test]
	fn reduced_resolution_shrinks_projected_size() -> anyhow::Result<()> {
		let full = projected_height_px(0.6, 20.0, 1080.0, 1.0);
		let half = projected_height_px(0.6, 20.0, 540.0, 1.0);
		assert!((full - 2.0 * half).abs() < 1e-3);
		Ok(())
	}
}
