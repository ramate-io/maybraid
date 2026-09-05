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

/// Optional coarser origin cells wrapping an inner footprint.
///
/// Each outer cell has edge length [`Self::cell_size`]. [`Self::rows`] Chebyshev
/// rings are placed around the previously covered footprint; cells that overlap
/// that footprint are omitted so grids abut without double-covering.
#[derive(Debug, Clone, PartialEq)]
pub struct OuterCellRing {
	/// Edge length of each outer cell in world units.
	pub cell_size: f32,
	/// Number of Chebyshev rows outside the current covered footprint.
	pub rows: i32,
}

/// One moving terrain-presentation stream on a globally aligned cell grid.
///
/// The High mesh occupies `high_inner_radius..=high_outer_radius` around the
/// current stream anchor. Cells remain generated for [`Self::cull_margin`]
/// beyond both edges so their High roots can stay warm during handoff.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainCellRing {
	/// Edge length of cells in this stream.
	pub cell_size: f32,
	/// Cascade mesh resolution for this stream (`2^res_2` samples per axis).
	pub res_2: u8,
	/// World-space lattice used to quantize the shared moving anchor.
	pub anchor_step: f32,
	/// Inner edge of the visible High annulus (`0` for the near stream).
	pub high_inner_radius: f32,
	/// Outer edge of the visible High annulus.
	pub high_outer_radius: f32,
	/// Empty retention band outside both High edges.
	pub cull_margin: f32,
}

impl TerrainCellRing {
	/// High / empty LOD band for a cell center relative to `anchor`.
	pub fn level_for(self, cell_center: Vec3, anchor: Vec3) -> lod::LodSceneLevel {
		let anchor = self.aligned_anchor(anchor);
		let delta = cell_center - anchor;
		let radius = delta.x.abs().max(delta.z.abs());
		if radius >= self.high_inner_radius && radius <= self.high_outer_radius {
			lod::LodSceneLevel::High
		} else {
			let retained_inner = (self.high_inner_radius - self.cull_margin).max(0.0);
			let retained_outer = self.high_outer_radius + self.cull_margin;
			if radius >= retained_inner && radius <= retained_outer {
				lod::LodSceneLevel::Medium
			} else {
				lod::LodSceneLevel::Low
			}
		}
	}

	fn retains_cell_center(self, cell_center: Vec3, anchor: Vec3) -> bool {
		self.level_for(cell_center, anchor) != lod::LodSceneLevel::Low
	}

	fn aligned_anchor(self, anchor: Vec3) -> Vec3 {
		let step = self.anchor_step.max(1e-3);
		Vec3::new((anchor.x / step).round() * step, 0.0, (anchor.z / step).round() * step)
	}
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
	/// Optional nested macro rings (increasing cell size), outside the fine grid.
	pub outer_rings: Vec<OuterCellRing>,
	/// Optional moving near / far / background streams.
	///
	/// When non-empty, origin-cell production uses these annuli instead of the
	/// bounded fine footprint and [`Self::outer_rings`].
	pub stream_rings: Vec<TerrainCellRing>,
}

impl Default for TerrainCellLayout {
	fn default() -> Self {
		Self {
			cell_size: TERRAIN_CELL_SIZE,
			vertical_half_extent: TERRAIN_CELL_VERTICAL_HALF_EXTENT,
			origin: TERRAIN_CELL_ORIGIN,
			extents: UVec2::new(TERRAIN_CELL_EXTENTS_XZ, TERRAIN_CELL_EXTENTS_XZ),
			outer_rings: Vec::new(),
			stream_rings: Vec::new(),
		}
	}
}

impl TerrainCellLayout {
	/// Fine-grid request AABB only (ignores [`Self::outer_rings`]).
	pub fn fine_request_region(&self) -> Aabb3d {
		let size = self.cell_size.max(1e-3);
		let vy = self.vertical_half_extent.max(size);
		let min = Vec3::new(self.origin.x as f32 * size, -vy, self.origin.y as f32 * size);
		let max = Vec3::new(
			(self.origin.x + self.extents.x as i32) as f32 * size,
			vy,
			(self.origin.y + self.extents.y as i32) as f32 * size,
		);
		Aabb3d::from_min_max(min, max)
	}

	/// Full request AABB including nested [`Self::outer_rings`] padding.
	pub fn request_region(&self) -> Aabb3d {
		if let Some(radius) = self
			.stream_rings
			.iter()
			.map(|ring| ring.high_outer_radius + ring.cull_margin)
			.max_by(f32::total_cmp)
		{
			let center = self.fine_region_center_xz();
			let vy = self.vertical_half_extent.max(radius);
			return Aabb3d::from_min_max(
				Vec3::new(center.x - radius, -vy, center.z - radius),
				Vec3::new(center.x + radius, vy, center.z + radius),
			);
		}
		let mut region = self.fine_request_region();
		for outer in &self.outer_rings {
			if outer.rows > 0 {
				let pad = outer.rows as f32 * outer.cell_size.max(1e-3);
				region = expand_aabb_xz(region, pad);
			}
		}
		region
	}

	/// World-space center of the request region on XZ (Y = 0).
	pub fn region_center_xz(&self) -> Vec3 {
		let region = self.fine_request_region();
		let min = Vec3::from(region.min);
		let max = Vec3::from(region.max);
		Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5)
	}

	fn fine_region_center_xz(&self) -> Vec3 {
		let region = self.fine_request_region();
		let min = Vec3::from(region.min);
		let max = Vec3::from(region.max);
		Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5)
	}

	/// Moving stream policy matching a cell edge length.
	pub fn stream_ring_for_cell_size(&self, cell_size: f32) -> Option<TerrainCellRing> {
		self.stream_rings
			.iter()
			.copied()
			.find(|ring| (ring.cell_size - cell_size).abs() < 1e-3)
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
		Self { cell_size: MACRO_CELL_SIZE, vertical_half_extent: TERRAIN_CELL_VERTICAL_HALF_EXTENT }
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
pub fn cell_coords_for_region(region: Aabb3d, cell_size: f32) -> impl Iterator<Item = (i32, i32)> {
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

/// Origin-cell [`OriginalId`]s covering `region` for a materialized layout.
///
/// Fine-grid cells come only from [`TerrainCellLayout::fine_request_region`],
/// not from the padded request AABB. Each [`TerrainCellLayout::outer_rings`]
/// entry then tiles only its own expanded frame (not the remaining pad), so
/// 2× cells do not fill the 4× ring and 160 m cells do not fill either ring.
pub fn origin_cell_ids_for_layout(layout: &TerrainCellLayout, region: Aabb3d) -> Vec<OriginalId> {
	if !layout.stream_rings.is_empty() {
		let min = Vec3::from(region.min);
		let max = Vec3::from(region.max);
		let anchor = Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5);
		let mut ids = std::collections::HashSet::new();
		for ring in &layout.stream_rings {
			let outer = ring.high_outer_radius + ring.cull_margin;
			let bounds = Aabb3d::from_min_max(
				Vec3::new(anchor.x - outer, region.min.y, anchor.z - outer),
				Vec3::new(anchor.x + outer, region.max.y, anchor.z + outer),
			);
			for (ix, iz) in cell_coords_for_region(bounds, ring.cell_size) {
				let cell = cell_bounds(ix, iz, ring.cell_size, layout.vertical_half_extent);
				let center = (Vec3::from(cell.min) + Vec3::from(cell.max)) * 0.5;
				if region.intersects(&cell) && ring.retains_cell_center(center, anchor) {
					ids.insert(OriginalId(Id::from_cell(cell)));
				}
			}
		}
		let mut ids: Vec<_> = ids.into_iter().collect();
		ids.sort_by(|a, b| a.0.cmp(&b.0));
		return ids;
	}

	let fine = layout.fine_request_region();
	let mut ids: Vec<OriginalId> = cell_coords_for_region(fine, layout.cell_size)
		.filter_map(|(ix, iz)| {
			let bounds = cell_bounds(ix, iz, layout.cell_size, layout.vertical_half_extent);
			region.intersects(&bounds).then(|| OriginalId(Id::from_cell(bounds)))
		})
		.collect();

	let mut covered = fine;
	for outer in &layout.outer_rings {
		if outer.rows <= 0 {
			continue;
		}
		let g = outer.cell_size.max(1e-3);
		let hole: std::collections::HashSet<(i32, i32)> =
			cell_coords_for_region(covered, g).collect();
		let expanded = expand_aabb_xz(covered, outer.rows as f32 * g);
		let outer_ids = cell_coords_for_region(expanded, g).filter_map(|(ix, iz)| {
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
		covered = expanded;
	}

	ids
}

/// Origin-cell [`OriginalId`]s covering `region`, using Universal [`TerrainCellLayout`].
///
/// Emits fine-grid cells plus nested [`TerrainCellLayout::outer_rings`] macro
/// cells that intersect `region` and do not overlap the previously covered
/// footprint. See [`origin_cell_ids_for_layout`].
pub fn original_ids_for_origin_cells<S>(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId>
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
	let Some(layout) = <S as SpatialIndex<TerrainCellLayout>>::get(spatial_index, Id::Universal)
	else {
		return Vec::new();
	};
	origin_cell_ids_for_layout(&layout.clone(), region)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn world_like_layout() -> TerrainCellLayout {
		let mut layout = TerrainCellLayout::default();
		layout.origin = IVec2::new(-16, -16);
		layout.extents = UVec2::new(32, 32);
		layout.outer_rings = vec![
			OuterCellRing { cell_size: 2.0 * TERRAIN_CELL_SIZE, rows: 2 },
			OuterCellRing { cell_size: 4.0 * TERRAIN_CELL_SIZE, rows: 1 },
		];
		layout
	}

	fn streamed_layout() -> TerrainCellLayout {
		let mut layout = TerrainCellLayout::default();
		layout.stream_rings = vec![
			TerrainCellRing {
				cell_size: TERRAIN_CELL_SIZE,
				res_2: 5,
				anchor_step: 4.0 * TERRAIN_CELL_SIZE,
				high_inner_radius: 0.0,
				high_outer_radius: 8.0 * TERRAIN_CELL_SIZE,
				cull_margin: 2.0 * TERRAIN_CELL_SIZE,
			},
			TerrainCellRing {
				cell_size: 2.0 * TERRAIN_CELL_SIZE,
				res_2: 4,
				anchor_step: 4.0 * TERRAIN_CELL_SIZE,
				high_inner_radius: 8.0 * TERRAIN_CELL_SIZE,
				high_outer_radius: 16.0 * TERRAIN_CELL_SIZE,
				cull_margin: 2.0 * TERRAIN_CELL_SIZE,
			},
		];
		layout
	}

	fn edge_len(id: &OriginalId) -> f32 {
		let bounds = id.0.origin_cell_bounds().expect("origin cell");
		bounds.max.x - bounds.min.x
	}

	fn count_edge(ids: &[OriginalId], expected: f32) -> usize {
		ids.iter().filter(|id| (edge_len(id) - expected).abs() < 1e-3).count()
	}

	#[test]
	fn padded_request_does_not_paint_fine_cells_on_macro_rings() {
		let layout = world_like_layout();
		let ids = origin_cell_ids_for_layout(&layout, layout.request_region());
		let fine = TERRAIN_CELL_SIZE;
		assert_eq!(count_edge(&ids, fine), 32 * 32);
		assert_eq!(count_edge(&ids, 2.0 * fine), 144);
		assert_eq!(count_edge(&ids, 4.0 * fine), 44);
		assert_eq!(ids.len(), 32 * 32 + 144 + 44);
	}

	#[test]
	fn outer_ring_query_does_not_emit_fine_cells() {
		let layout = world_like_layout();
		let fine_region = layout.fine_request_region();
		let pad = 4.0 * TERRAIN_CELL_SIZE;
		let outer_query = Aabb3d::from_min_max(
			Vec3::new(fine_region.max.x + pad * 0.25, fine_region.min.y, 0.0),
			Vec3::new(fine_region.max.x + pad * 0.75, fine_region.max.y, pad),
		);
		let ids = origin_cell_ids_for_layout(&layout, outer_query);
		assert_eq!(count_edge(&ids, TERRAIN_CELL_SIZE), 0);
		assert!(!ids.is_empty());
	}

	#[test]
	fn moving_streams_use_high_and_retention_annuli() {
		let layout = streamed_layout();
		let radius = 20.0 * TERRAIN_CELL_SIZE;
		let region = Aabb3d::from_min_max(
			Vec3::new(-radius, -layout.vertical_half_extent, -radius),
			Vec3::new(radius, layout.vertical_half_extent, radius),
		);
		let ids = origin_cell_ids_for_layout(&layout, region);
		assert!(count_edge(&ids, TERRAIN_CELL_SIZE) > 0);
		assert!(count_edge(&ids, 2.0 * TERRAIN_CELL_SIZE) > 0);

		let coarse_inside_hole = cell_bounds(0, 0, 2.0 * TERRAIN_CELL_SIZE, 10.0);
		assert!(!ids.iter().any(|id| id.0 == Id::from_cell(coarse_inside_hole)));
	}

	#[test]
	fn stream_level_is_high_then_empty_then_cull() {
		let ring = streamed_layout().stream_rings[0];
		assert_eq!(ring.level_for(Vec3::ZERO, Vec3::ZERO), lod::LodSceneLevel::High);
		assert_eq!(
			ring.level_for(Vec3::X * 9.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::Medium
		);
		assert_eq!(
			ring.level_for(Vec3::X * 11.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::Low
		);
	}
}
