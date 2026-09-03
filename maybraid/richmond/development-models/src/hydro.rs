//! Skip development cells that overlap Marazion hydro primitives.

use durham_terrain_models::{origin_cell_ids_for_layout, TerrainCellLayout, TerrainEntryStore};
use lod::gen::OriginalId;
use marazion_watersheds::{WaterFill, WaterSurface};
use procedural_common::Bounds2;

const SITE_SAMPLE_SIDE: usize = 9;
const SITE_HEIGHT_QUANTILE: f32 = 0.95;

/// True when `bounds` overlaps any hydro primitive support in `fills`.
pub fn hydro_overlaps_xz(fills: &[WaterFill], bounds: Bounds2) -> bool {
	for fill in fills {
		match &fill.surface {
			WaterSurface::Hydro { complex } => {
				for node in &complex.hydrology {
					if node.correction_intersects(bounds) {
						return true;
					}
				}
			}
			WaterSurface::Flat { region, .. } => {
				let c = bounds.center();
				if region.sdf(c) <= 0.0 {
					return true;
				}
				let corners = [
					bounds.min,
					bevy::math::Vec2::new(bounds.max.x, bounds.min.y),
					bounds.max,
					bevy::math::Vec2::new(bounds.min.x, bounds.max.y),
				];
				if corners.iter().any(|p| region.sdf(*p) <= 0.0) {
					return true;
				}
			}
		}
	}
	false
}

/// True when any stored terrain cell overlapping `cell` has hydro that intersects `bounds`.
pub fn terrain_hydro_overlaps(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	cell: bevy::math::bounding::Aabb3d,
	bounds: Bounds2,
) -> bool {
	for OriginalId(id) in origin_cell_ids_for_layout(layout, cell) {
		let Some(terrain) = store.terrain(id) else {
			continue;
		};
		if hydro_overlaps_xz(&terrain.marazion_fills, bounds) {
			return true;
		}
	}
	false
}

/// Height sample from composed post-Marazion terrain, if the covering cell is stored.
pub fn composed_height_at(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	x: f32,
	z: f32,
) -> Option<f32> {
	store.composed_height_at(layout, x, z)
}

/// Robust high composed elevation over a yawed rectangular support.
///
/// A dense grid over the complete pad influence catches uphill terrain outside
/// the flatten core. The 95th percentile sits close to that local high without
/// letting one narrow terrain spike lift the entire terrace.
pub fn composed_height_upper_on_rect(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	center: bevy::math::Vec2,
	half: bevy::math::Vec2,
	yaw: f32,
) -> Option<f32> {
	let (sin, cos) = yaw.sin_cos();
	let mut heights = Vec::with_capacity(SITE_SAMPLE_SIDE * SITE_SAMPLE_SIDE);
	for iz in 0..SITE_SAMPLE_SIDE {
		for ix in 0..SITE_SAMPLE_SIDE {
			let u = ix as f32 / (SITE_SAMPLE_SIDE - 1) as f32;
			let v = iz as f32 / (SITE_SAMPLE_SIDE - 1) as f32;
			let local =
				bevy::math::Vec2::new(-half.x + 2.0 * half.x * u, -half.y + 2.0 * half.y * v);
			let p = center
				+ bevy::math::Vec2::new(
					cos * local.x + sin * local.y,
					-sin * local.x + cos * local.y,
				);
			if let Some(h) = composed_height_at(store, layout, p.x, p.y) {
				heights.push(h);
			}
		}
	}
	upper_quantile(&mut heights, SITE_HEIGHT_QUANTILE)
}

fn upper_quantile(values: &mut [f32], quantile: f32) -> Option<f32> {
	if values.is_empty() {
		return None;
	}
	values.sort_by(f32::total_cmp);
	let index = ((values.len() - 1) as f32 * quantile.clamp(0.0, 1.0)).ceil() as usize;
	values.get(index).copied()
}

#[cfg(test)]
mod tests {
	use super::*;
	use jersey_terrain_stamps::{CircleRegion, Region2D};

	#[test]
	fn flat_fill_overlaps_center() {
		let fills = vec![WaterFill {
			surface: WaterSurface::Flat {
				level: 10.0,
				region: Region2D::Circle(CircleRegion {
					center: bevy::math::Vec2::ZERO,
					radius: 20.0,
				}),
			},
		}];
		let hit = Bounds2::from_xz(-5.0, -5.0, 5.0, 5.0);
		let miss = Bounds2::from_xz(80.0, 80.0, 90.0, 90.0);
		assert!(hydro_overlaps_xz(&fills, hit));
		assert!(!hydro_overlaps_xz(&fills, miss));
	}

	#[test]
	fn upper_site_height_ignores_one_narrow_spike() -> anyhow::Result<()> {
		let mut heights: Vec<f32> = (0..80).map(|i| i as f32).collect();
		heights.push(1000.0);
		let high = upper_quantile(&mut heights, SITE_HEIGHT_QUANTILE)
			.ok_or_else(|| anyhow::anyhow!("expected a quantile"))?;
		anyhow::ensure!(high >= 70.0, "site height should remain near the local high: {high}");
		anyhow::ensure!(high < 1000.0, "one spike should not lift the terrace: {high}");
		Ok(())
	}
}
