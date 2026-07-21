//! Pre-pocket grid layout + cells ([RFC-127 §3.1.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#311-pre-pocket-cells)).

use crate::terrain::cell::{cell_coords_for_region, universal_bounds};
use crate::terrain::jersey::shared::OffsetControllerGrid;
use crate::terrain::marazion::config::MarazionWatershedConfigs;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{Vec2, Vec3};
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use marazion_watersheds::{PrePocket, DEFAULT_PRE_POCKET_PITCH};
use procedural_common::Bounds2;

/// World-anchored pre-pocket controller grid.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct PrePocketLayout {
	pub grid: OffsetControllerGrid,
}

impl Default for PrePocketLayout {
	fn default() -> Self {
		Self {
			grid: OffsetControllerGrid::new(DEFAULT_PRE_POCKET_PITCH, Vec2::ZERO),
		}
	}
}

impl PrePocketLayout {
	pub fn cell_bounds(&self, ix: i32, iz: i32) -> Aabb3d {
		self.grid.cell_bounds(ix, iz)
	}
}

/// Sync layout pitch/origin from watershed configs when bootstrapping.
pub fn pre_pocket_layout_from_configs(configs: &MarazionWatershedConfigs) -> PrePocketLayout {
	PrePocketLayout {
		grid: OffsetControllerGrid::new(
			configs.pre_pocket.pitch.max(1.0),
			configs.pre_pocket.origin,
		),
	}
}

pub trait BootstrapPrePocketLayout {
	fn bootstrap_pre_pocket_layout(&self) -> PrePocketLayout;
}

impl<S> GenerationScheme<S> for PrePocketLayout
where
	S: BootstrapPrePocketLayout,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((
			spatial_index.bootstrap_pre_pocket_layout(),
			universal_bounds(),
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

/// One pre-pocket tile with its chosen pocket pitch.
#[derive(Debug, Clone, Component)]
pub struct PrePocketCell {
	pub cell: Aabb3d,
	pub pre: PrePocket,
}

pub fn original_ids_for_pre_pocket_cells<S>(
	spatial_index: &mut S,
	region: Aabb3d,
) -> Vec<OriginalId>
where
	S: GeneratingSpatialIndex<PrePocketLayout>,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	if GeneratingSpatialIndex::<PrePocketLayout>::get_or_generate(
		spatial_index,
		Id::Universal,
		&lod_ref,
	)
	.is_none()
	{
		return Vec::new();
	}
	let Some(layout) = <S as SpatialIndex<PrePocketLayout>>::get(spatial_index, Id::Universal)
	else {
		return Vec::new();
	};
	let grid = layout.grid.clone();
	let grid_region = grid.region_in_grid_space(region);
	cell_coords_for_region(grid_region, grid.cell_size)
		.map(|(ix, iz)| OriginalId(Id::from_cell(grid.cell_bounds(ix, iz))))
		.filter(|OriginalId(id)| {
			id.origin_cell_bounds().is_some_and(|b| region.intersects(&b))
		})
		.collect()
}

impl<S> GenerationScheme<S> for PrePocketCell
where
	S: GeneratingSpatialIndex<MarazionWatershedConfigs> + GeneratingSpatialIndex<PrePocketLayout>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_pre_pocket_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let cell = id.origin_cell_bounds()?;
		let configs = GeneratingSpatialIndex::<MarazionWatershedConfigs>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let cx = (cell.min.x + cell.max.x) * 0.5;
		let cz = (cell.min.z + cell.max.z) * 0.5;
		let mut params = configs.pre_pocket;
		params.seed = configs.seed;
		let pre = PrePocket::containing(cx, cz, &params);
		Some((Self { cell, pre }, cell))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

/// Build an AABB for a pocket tile (XZ from [`PrePocket`], Y from parent cell).
pub fn pocket_aabb(pre: &PrePocket, px: u32, pz: u32, vy_min: f32, vy_max: f32) -> Aabb3d {
	aabb_from_bounds2(pre.pocket_bounds(px, pz), vy_min, vy_max)
}

pub fn aabb_from_bounds2(b: Bounds2, vy_min: f32, vy_max: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(b.min.x, vy_min, b.min.y),
		Vec3::new(b.max.x, vy_max, b.max.y),
	)
}
