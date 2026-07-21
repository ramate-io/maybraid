//! Shared pre-watershed height sampling for Marazion lake leaves.

use crate::terrain::cell::{cell_bounds, TerrainCellLayout};
use crate::terrain::PreWatershedTerrain;
use lod::gen::{GeneratingSpatialIndex, Id};
use lod::lod_ref::LodRef;

pub(crate) fn pre_watershed_height_at<S>(
	spatial_index: &mut S,
	x: f32,
	z: f32,
	lod_ref: &LodRef,
) -> Option<f32>
where
	S: GeneratingSpatialIndex<PreWatershedTerrain> + GeneratingSpatialIndex<TerrainCellLayout>,
{
	let layout = GeneratingSpatialIndex::<TerrainCellLayout>::get_one_or_generate(
		spatial_index,
		Id::Universal,
		lod_ref,
	)?;
	let size = layout.cell_size.max(1e-3);
	let ix = (x / size).floor() as i32;
	let iz = (z / size).floor() as i32;
	let cell = cell_bounds(ix, iz, size, layout.vertical_half_extent);
	let id = Id::from_cell(cell);
	let pre = GeneratingSpatialIndex::<PreWatershedTerrain>::get_one_or_generate(
		spatial_index,
		id,
		lod_ref,
	)?;
	Some(pre.sdf.terrain.height_at_with_all_modulations(x, z))
}
