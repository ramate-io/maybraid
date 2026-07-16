//! Per-macro-cell grading graph built with a hysteresis path over base noise.

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{
	cell_bounds, cell_coords_for_region_inclusive, TerrainCellLayout,
};
use crate::terrain::presentation::TerrainPresentationAssets;
use crate::terrain::region::grading::RegionGradingModulation;
use crate::terrain::region::{RectRegion, Region2D};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, SeededHash};

/// One graded corridor segment (polyline endpoints + elevations).
#[derive(Debug, Clone)]
pub struct GradingSegment {
	pub start: Vec2,
	pub end: Vec2,
	pub start_elevation: f32,
	pub end_elevation: f32,
	pub half_width: f32,
}

impl GradingSegment {
	pub fn to_modulation(&self) -> RegionGradingModulation {
		let mid = (self.start + self.end) * 0.5;
		let len = self.start.distance(self.end).max(1.0);
		RegionGradingModulation::new(
			Region2D::Rect(RectRegion {
				center: mid,
				half_extents: Vec2::new(len * 0.5, self.half_width),
				round: 0.05,
			}),
			self.start,
			self.start_elevation,
			self.end,
			self.end_elevation,
			None,
			self.half_width.max(0.2),
			self.half_width.max(0.1) * 0.5,
		)
	}
}

/// Macro-cell grading structure: hysteresis path + elevation samples from base noise.
#[derive(Debug, Clone, Component)]
pub struct GradingGraph {
	pub cell: Aabb3d,
	pub path: Vec<Vec2>,
	pub segments: Vec<GradingSegment>,
}

impl GradingGraph {
	pub fn modulations(&self) -> impl Iterator<Item = RegionGradingModulation> + '_ {
		self.segments.iter().map(GradingSegment::to_modulation)
	}

	/// Build grading for one macro origin cell from base noise samples.
	pub fn from_macro_cell(
		cell: Aabb3d,
		base: &BaseTerrainNoise,
		seed: u32,
		macro_cell_size: f32,
	) -> Self {
		let min = Vec2::new(cell.min.x, cell.min.z);
		let max = Vec2::new(cell.max.x, cell.max.z);
		let bounds = Bounds2::new(min, max);
		let cell_size = macro_cell_size.max(1e-3);
		let ix = (cell.min.x / cell_size).floor() as i32;
		let iz = (cell.min.z / cell_size).floor() as i32;
		let noise = SeededHash::new(seed);

		let n0 = noise.unit_i32(ix, iz);
		let n1 = noise.unit_i32(ix.wrapping_add(3), iz.wrapping_add(7));
		let start = Vec2::new(
			min.x + (0.15 + 0.2 * n0) * (max.x - min.x),
			min.y + (0.2 + 0.25 * n1) * (max.y - min.y),
		);
		let end = Vec2::new(
			min.x + (0.65 + 0.2 * n1) * (max.x - min.x),
			min.y + (0.55 + 0.25 * n0) * (max.y - min.y),
		);

		let graph = HysteresisGraph::degree1(
			bounds,
			seed.wrapping_add(ix as u32),
			start,
			end,
			&HysteresisConfig::default(),
		);
		let path = graph.primary_polyline();
		let mut segments = Vec::new();
		if path.len() >= 2 {
			let a = path[0];
			let b = path[path.len() - 1];
			segments.push(GradingSegment {
				start: a,
				end: b,
				start_elevation: base.height_at(a.x, a.y),
				end_elevation: base.height_at(b.x, b.y),
				half_width: 1.0 + 2.0 * n0,
			});
		}

		Self { cell, path, segments }
	}
}

impl<S> GenerationScheme<S> for GradingGraph
where
	S: GeneratingSpatialIndex<BaseTerrainNoise>
		+ GeneratingSpatialIndex<TerrainCellLayout>
		+ GeneratingSpatialIndex<TerrainPresentationAssets>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
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
		let macro_layout = layout.macro_layout();
		// Closed/inclusive: face-adjacent macros whose softmask can still spill in.
		cell_coords_for_region_inclusive(region, macro_layout.cell_size)
			.map(|(ix, iz)| {
				let bounds = cell_bounds(
					ix,
					iz,
					macro_layout.cell_size,
					macro_layout.vertical_half_extent,
				);
				OriginalId(Id::from_cell(bounds))
			})
			.collect()
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
		GeneratingSpatialIndex::<TerrainPresentationAssets>::get_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let seed = <S as SpatialIndex<TerrainPresentationAssets>>::get(spatial_index, Id::Universal)?
			.config
			.seed;
		GeneratingSpatialIndex::<TerrainCellLayout>::get_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let cell_size = <S as SpatialIndex<TerrainCellLayout>>::get(spatial_index, Id::Universal)?
			.macro_cell_size();
		let graph = Self::from_macro_cell(bounds, &base, seed, cell_size);
		Some((graph, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
