//! Per-macro-cell region stamps placed via hysteresis point search.

use crate::terrain::cell::{
	cell_bounds, cell_coords_for_region_inclusive, TerrainCellLayout,
	MACRO_CELL_MODULATION_APRON,
};
use crate::terrain::presentation::TerrainPresentationAssets;
use crate::terrain::region::affine::RegionAffineModulation;
use crate::terrain::region::{CellApron, CircleRegion, Region2D, RegionNoise};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, SeededHash};

/// One stamp candidate (center + modulation params).
#[derive(Debug, Clone)]
pub struct RegionStamp {
	pub center: Vec2,
	pub radius: f32,
	pub inner_scale: f32,
	pub inner_offset: f32,
}

impl RegionStamp {
	pub fn to_modulation(&self, seed: u32, cell: Aabb3d) -> RegionAffineModulation {
		RegionAffineModulation::new(
			Region2D::Circle(CircleRegion { center: self.center, radius: self.radius }),
			self.inner_scale,
			self.inner_offset,
			self.radius * 0.15,
			self.radius * 0.2,
		)
		.with_noise(RegionNoise::from_seed(seed, 0.2, 2.0))
		.with_cell_apron(CellApron::from_aabb(cell, MACRO_CELL_MODULATION_APRON))
	}
}

/// Macro-cell stamp set selected by hysteresis search.
#[derive(Debug, Clone, Component)]
pub struct RegionStamps {
	pub cell: Aabb3d,
	pub stamps: Vec<RegionStamp>,
}

impl RegionStamps {
	pub fn modulations(&self, seed: u32) -> impl Iterator<Item = RegionAffineModulation> + '_ {
		let cell = self.cell;
		self.stamps.iter().map(move |s| s.to_modulation(seed, cell))
	}

	/// Place stamps inside one macro origin cell via radial hysteresis tips.
	pub fn from_macro_cell(cell: Aabb3d, seed: u32, macro_cell_size: f32) -> Self {
		let cell_size = macro_cell_size.max(1e-3);
		let ix = (cell.min.x / cell_size).floor() as i32;
		let iz = (cell.min.z / cell_size).floor() as i32;
		let noise = SeededHash::new(seed);
		let bounds = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
		let count = 2 + (noise.unit_i32(ix, iz) * 3.0).floor() as usize;
		let points = HysteresisGraph::radial_tips(
			bounds,
			seed.wrapping_add((ix as u32).wrapping_mul(31).wrapping_add(iz as u32)),
			count,
			&HysteresisConfig::default(),
		);

		let stamps = points
			.into_iter()
			.enumerate()
			.map(|(i, center)| {
				let n = noise.unit_i32(ix.wrapping_add(i as i32), iz);
				RegionStamp {
					center,
					radius: 40.0 + 50.0 * n,
					inner_scale: 0.45 + 0.2 * n,
					inner_offset: -1.2 - n,
				}
			})
			.collect();

		Self { cell, stamps }
	}
}

impl<S> GenerationScheme<S> for RegionStamps
where
	S: GeneratingSpatialIndex<TerrainCellLayout>
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
		let stamps = Self::from_macro_cell(bounds, seed, cell_size);
		Some((stamps, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
