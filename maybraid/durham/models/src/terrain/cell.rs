//! Terrain cell size and origin tiling helpers.

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{IVec2, UVec2, Vec3};
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;

/// Naturescapes cascade `min_size`.
pub const NATURESCAPES_MIN_SIZE: f32 = 20.0;

/// Naturescapes `grid_multiple_2` (chunk size = [`NATURESCAPES_MIN_SIZE`] × 2^this).
pub const NATURESCAPES_GRID_MULTIPLE_2: u8 = 3;

/// Naturescapes `grid_radius` X/Z (inclusive range is `[-r, r]` → `2r + 1` cells).
pub const NATURESCAPES_GRID_RADIUS_XZ: i32 = 12;

/// Default edge length of a procedural terrain origin cell (world units).
///
/// Matches naturescapes grid chunk size: `min_size * 2^grid_multiple_2` = 20 × 8.
pub const TERRAIN_CELL_SIZE: f32 =
	NATURESCAPES_MIN_SIZE * (1_u32 << NATURESCAPES_GRID_MULTIPLE_2) as f32;

/// Macro-cell edge length (`4 ×` terrain cell). Used by jersey guillotine step windows.
pub const MACRO_CELL_SIZE: f32 = TERRAIN_CELL_SIZE * 4.0;

/// Mesh overflow past each Terrain cell face, in voxels at the cell's `res_2`.
///
/// `0` keeps the cascade chunk flush with the cell AABB (no shared skirt).
/// Non-zero values expand XZ by that many sample pitches (and Y by
/// [`TERRAIN_MESH_PAD_Y_SLOPE`] × that pad) so neighbors share a strip.
pub const TERRAIN_MESH_PAD_VOXELS: f32 = 0.0;

/// Extra Y overflow as a multiple of the XZ pad (covers steep ridge crests).
pub const TERRAIN_MESH_PAD_Y_SLOPE: f32 = 4.0;

/// Default cell count along +X / +Z (`2 * grid_radius + 1`).
pub const TERRAIN_CELL_EXTENTS_XZ: u32 = (2 * NATURESCAPES_GRID_RADIUS_XZ + 1) as u32;

/// Default cell-grid origin so the request region is centered like naturescapes at the world origin.
pub const TERRAIN_CELL_ORIGIN: IVec2 =
	IVec2::new(-NATURESCAPES_GRID_RADIUS_XZ, -NATURESCAPES_GRID_RADIUS_XZ);

/// Default vertical half-extent so cells cover naturescapes-scale heightfields
/// (`height_scale=500`, bedrock at `-4 * height_scale`).
pub const TERRAIN_CELL_VERTICAL_HALF_EXTENT: f32 = 2000.0;

/// Large AABB for universal (`Id::Universal`) generation deps.
pub fn universal_bounds() -> Aabb3d {
	Aabb3d::from_min_max(Vec3::splat(-1_000_000.0), Vec3::splat(1_000_000.0))
}

/// Optional coarser origin cells wrapping the fine [`TerrainCellLayout`] footprint.
///
/// Each outer cell has edge length [`Self::cell_size`] (typically
/// `inner_radius_cells * fine_cell_size`). [`Self::rows`] Chebyshev rings are
/// placed around the fine region; outer cells that overlap the fine footprint
/// are omitted so the two grids abut without double-covering.
#[derive(Debug, Clone, PartialEq)]
pub struct OuterCellRing {
	/// Edge length of each outer cell in world units.
	pub cell_size: f32,
	/// Number of Chebyshev rows outside the fine footprint.
	pub rows: i32,
}

/// Layout for tiling terrain origin cells in the XZ plane.
///
/// Materialized once under [`Id::Universal`] via [`GenerationScheme`].
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct TerrainCellLayout {
	/// Edge length of each origin cell in world units.
	pub cell_size: f32,
	/// Half-extent along Y for cell bounds / SDF sampling volumes.
	pub vertical_half_extent: f32,
	/// Cell-grid coordinates of the min corner (XZ).
	pub origin: IVec2,
	/// Number of cells along +X and +Z from [`Self::origin`].
	pub extents: UVec2,
	/// Optional macro cells wrapping the fine grid (larger XZ, same Y half-extent).
	pub outer_ring: Option<OuterCellRing>,
}

impl Default for TerrainCellLayout {
	fn default() -> Self {
		Self {
			cell_size: TERRAIN_CELL_SIZE,
			vertical_half_extent: TERRAIN_CELL_VERTICAL_HALF_EXTENT,
			origin: TERRAIN_CELL_ORIGIN,
			extents: UVec2::new(TERRAIN_CELL_EXTENTS_XZ, TERRAIN_CELL_EXTENTS_XZ),
			outer_ring: None,
		}
	}
}

impl TerrainCellLayout {
	/// Fine-grid request AABB only (ignores [`Self::outer_ring`]).
	pub fn fine_request_region(&self) -> Aabb3d {
		let size = self.cell_size.max(1e-3);
		let vy = self.vertical_half_extent.max(size);
		let min = Vec3::new(
			self.origin.x as f32 * size,
			-vy,
			self.origin.y as f32 * size,
		);
		let max = Vec3::new(
			(self.origin.x + self.extents.x as i32) as f32 * size,
			vy,
			(self.origin.y + self.extents.y as i32) as f32 * size,
		);
		Aabb3d::from_min_max(min, max)
	}

	/// Full request AABB including any [`Self::outer_ring`] padding.
	pub fn request_region(&self) -> Aabb3d {
		let fine = self.fine_request_region();
		match &self.outer_ring {
			Some(outer) if outer.rows > 0 => {
				let pad = outer.rows as f32 * outer.cell_size.max(1e-3);
				expand_aabb_xz(fine, pad)
			}
			_ => fine,
		}
	}

	/// World-space center of the request region on XZ (Y = 0).
	pub fn region_center_xz(&self) -> Vec3 {
		let region = self.request_region();
		let min = Vec3::from(region.min);
		let max = Vec3::from(region.max);
		Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5)
	}

	/// Macro-cell edge length, preserving the default `MACRO / TERRAIN` ratio.
	pub fn macro_cell_size(&self) -> f32 {
		self.cell_size * (MACRO_CELL_SIZE / TERRAIN_CELL_SIZE)
	}

	/// Macro tiling params derived from this layout.
	pub fn macro_layout(&self) -> MacroCellLayout {
		MacroCellLayout {
			cell_size: self.macro_cell_size(),
			vertical_half_extent: self.vertical_half_extent,
		}
	}
}

/// Bootstrap source used only when first materializing [`TerrainCellLayout`] at
/// [`Id::Universal`]. Consumers should depend on
/// [`lod::gen::GeneratingSpatialIndex`]`<TerrainCellLayout>` instead.
pub trait BootstrapTerrainCellLayout {
	fn bootstrap_terrain_cell_layout(&self) -> TerrainCellLayout;
}

impl<S> GenerationScheme<S> for TerrainCellLayout
where
	S: BootstrapTerrainCellLayout,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((spatial_index.bootstrap_terrain_cell_layout(), universal_bounds()))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

/// Layout for macro-scale tiling (jersey stamp size defaults).
///
/// Derived from [`TerrainCellLayout::macro_layout`]; not a separate generation type.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroCellLayout {
	pub cell_size: f32,
	pub vertical_half_extent: f32,
}

impl Default for MacroCellLayout {
	fn default() -> Self {
		Self {
			cell_size: MACRO_CELL_SIZE,
			vertical_half_extent: TERRAIN_CELL_VERTICAL_HALF_EXTENT,
		}
	}
}

/// Build an origin-cell AABB from integer cell coordinates on the XZ plane.
pub fn cell_bounds(ix: i32, iz: i32, cell_size: f32, vertical_half_extent: f32) -> Aabb3d {
	let size = cell_size.max(1e-3);
	let vy = vertical_half_extent.max(size);
	let min = Vec3::new(ix as f32 * size, -vy, iz as f32 * size);
	let max = Vec3::new((ix + 1) as f32 * size, vy, (iz + 1) as f32 * size);
	Aabb3d::from_min_max(min, max)
}

/// Integer cell coordinates covering a region on XZ (Y ignored for tiling).
///
/// Uses half-open style on the max edge (`ceil(max/size) - 1`), so a query whose
/// max lies exactly on a cell boundary does not include the next cell.
pub fn cell_coords_for_region(
	region: Aabb3d,
	cell_size: f32,
) -> impl Iterator<Item = (i32, i32)> {
	let size = cell_size.max(1e-3);
	let min_x = (region.min.x / size).floor() as i32;
	let max_x = (region.max.x / size).ceil() as i32 - 1;
	let min_z = (region.min.z / size).floor() as i32;
	let max_z = (region.max.z / size).ceil() as i32 - 1;
	(min_x..=max_x).flat_map(move |ix| (min_z..=max_z).map(move |iz| (ix, iz)))
}

/// Cell coordinates for closed AABB overlap with half-open tiles `[i·s,(i+1)·s]`.
pub fn cell_coords_for_region_inclusive(
	region: Aabb3d,
	cell_size: f32,
) -> impl Iterator<Item = (i32, i32)> {
	cell_coords_for_region_inclusive_halo(region, cell_size, 0)
}

/// Closed overlap plus a Moore halo of `halo` cells (for softmask / apron reach).
///
/// Closed face rule: `max >= i·s` and `min <= (i+1)·s`, then expand by `halo`.
pub fn cell_coords_for_region_inclusive_halo(
	region: Aabb3d,
	cell_size: f32,
	halo: i32,
) -> impl Iterator<Item = (i32, i32)> {
	let size = cell_size.max(1e-3);
	let halo = halo.max(0);
	let min_x = (region.min.x / size).ceil() as i32 - 1 - halo;
	let max_x = (region.max.x / size).floor() as i32 + halo;
	let min_z = (region.min.z / size).ceil() as i32 - 1 - halo;
	let max_z = (region.max.z / size).floor() as i32 + halo;
	(min_x..=max_x).flat_map(move |ix| (min_z..=max_z).map(move |iz| (ix, iz)))
}

/// Expand an AABB on XZ only (Y unchanged).
pub fn expand_aabb_xz(region: Aabb3d, pad: f32) -> Aabb3d {
	expand_aabb_xz_y(region, pad, 0.0)
}

/// Expand an AABB with independent XZ and Y pads.
pub fn expand_aabb_xz_y(region: Aabb3d, pad_xz: f32, pad_y: f32) -> Aabb3d {
	let pad_xz = pad_xz.max(0.0);
	let pad_y = pad_y.max(0.0);
	Aabb3d::from_min_max(
		Vec3::new(region.min.x - pad_xz, region.min.y - pad_y, region.min.z - pad_xz),
		Vec3::new(region.max.x + pad_xz, region.max.y + pad_y, region.max.z + pad_xz),
	)
}

/// Origin-cell [`OriginalId`]s covering `region`, using Universal [`TerrainCellLayout`].
///
/// Emits fine-grid cells plus any [`TerrainCellLayout::outer_ring`] macro cells
/// that intersect `region` and do not overlap the fine footprint.
pub fn original_ids_for_origin_cells<S>(
	spatial_index: &mut S,
	region: Aabb3d,
) -> Vec<OriginalId>
where
	S: GeneratingSpatialIndex<TerrainCellLayout>,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	if GeneratingSpatialIndex::<TerrainCellLayout>::get_or_generate(
		spatial_index,
		Id::Universal,
		&lod_ref,
	)
	.is_none()
	{
		return Vec::new();
	}
	let Some(layout) =
		<S as SpatialIndex<TerrainCellLayout>>::get(spatial_index, Id::Universal)
	else {
		return Vec::new();
	};
	let layout = layout.clone();
	let mut ids: Vec<OriginalId> = cell_coords_for_region(region, layout.cell_size)
		.map(|(ix, iz)| {
			let bounds = cell_bounds(ix, iz, layout.cell_size, layout.vertical_half_extent);
			OriginalId(Id::from_cell(bounds))
		})
		.filter(|OriginalId(id)| {
			id.origin_cell_bounds().is_some_and(|b| region.intersects(&b))
		})
		.collect();

	if let Some(outer) = &layout.outer_ring {
		let g = outer.cell_size.max(1e-3);
		let fine = layout.fine_request_region();
		// Half-open fine footprint → integer hole on the macro lattice (abutting,
		// not face-touch filtered, so boundary tiles are kept).
		let hole: std::collections::HashSet<(i32, i32)> =
			cell_coords_for_region(fine, g).collect();
		let outer_ids = cell_coords_for_region(region, g).filter_map(|(ix, iz)| {
			if hole.contains(&(ix, iz)) {
				return None;
			}
			let bounds = cell_bounds(ix, iz, g, layout.vertical_half_extent);
			if !region.intersects(&bounds) {
				return None;
			}
			Some(OriginalId(Id::from_cell(bounds)))
		});
		ids.extend(outer_ids);
	}

	ids
}
