//! Per-jersey-cell height view over Universal [`BaseTerrainNoise`].

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{original_ids_for_jersey_cells, JerseyStampCellLayout};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;

const SAMPLE_GRID: u32 = 5;

/// Local height oracle and cheap stats for one jersey stamp cell.
#[derive(Debug, Clone, Component)]
pub struct CellTerrainNoise {
	pub cell: Aabb3d,
	pub seed: u32,
	pub mean: f32,
	pub range: f32,
	pub center_height: f32,
	pub corner_mean: f32,
	base: BaseTerrainNoise,
}

impl CellTerrainNoise {
	/// Sample base noise over the cell and cache mean / range / center vs corners.
	pub fn from_base(cell: Aabb3d, base: BaseTerrainNoise) -> Self {
		let (mean, range, center_height, corner_mean) = sample_cell_stats(&base, cell);
		Self {
			cell,
			seed: base.seed,
			mean,
			range,
			center_height,
			corner_mean,
			base,
		}
	}

	pub fn height_at(&self, x: f32, z: f32) -> f32 {
		self.base.height_at(x, z)
	}

	/// Center minus corner mean: negative → depression, positive → ridge.
	pub fn relief_delta(&self) -> f32 {
		self.center_height - self.corner_mean
	}
}

fn sample_cell_stats(
	base: &BaseTerrainNoise,
	cell: Aabb3d,
) -> (f32, f32, f32, f32) {
	let min_x = cell.min.x;
	let max_x = cell.max.x;
	let min_z = cell.min.z;
	let max_z = cell.max.z;
	let n = SAMPLE_GRID.max(2);
	let mut sum = 0.0;
	let mut min_h = f32::INFINITY;
	let mut max_h = f32::NEG_INFINITY;
	let denom = (n - 1) as f32;
	for iz in 0..n {
		for ix in 0..n {
			let u = ix as f32 / denom;
			let v = iz as f32 / denom;
			let x = min_x + (max_x - min_x) * u;
			let z = min_z + (max_z - min_z) * v;
			let h = base.height_at(x, z);
			sum += h;
			min_h = min_h.min(h);
			max_h = max_h.max(h);
		}
	}
	let count = (n * n) as f32;
	let mean = sum / count;
	let range = (max_h - min_h).max(0.0);
	let cx = (min_x + max_x) * 0.5;
	let cz = (min_z + max_z) * 0.5;
	let center_height = base.height_at(cx, cz);
	let corner_mean = (base.height_at(min_x, min_z)
		+ base.height_at(max_x, min_z)
		+ base.height_at(min_x, max_z)
		+ base.height_at(max_x, max_z))
		* 0.25;
	(mean, range, center_height, corner_mean)
}

impl<S> GenerationScheme<S> for CellTerrainNoise
where
	S: GeneratingSpatialIndex<BaseTerrainNoise> + GeneratingSpatialIndex<JerseyStampCellLayout>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_jersey_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		GeneratingSpatialIndex::<BaseTerrainNoise>::get_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let base =
			<S as SpatialIndex<BaseTerrainNoise>>::get(spatial_index, Id::Universal)?.clone();
		Some((Self::from_base(bounds, base), bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
