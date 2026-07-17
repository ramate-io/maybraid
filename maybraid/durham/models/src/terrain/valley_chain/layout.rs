//! Uniform controller-grid layout for ValleyChain.

use crate::terrain::cell::{
	cell_bounds, cell_coords_for_region, universal_bounds, MACRO_CELL_SIZE,
	TERRAIN_CELL_VERTICAL_HALF_EXTENT,
};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;

/// Default controller-cell edge length (world units): `4 ×` macro / jersey stamp.
pub const VALLEY_CHAIN_CONTROLLER_CELL_SIZE: f32 = MACRO_CELL_SIZE * 4.0;

/// Layout for ValleyChain controller cells (larger than jersey stamp cells).
///
/// Materialized once under [`Id::Universal`] via [`GenerationScheme`].
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct JerseyValleyChainControllerLayout {
	pub cell_size: f32,
	pub vertical_half_extent: f32,
}

impl Default for JerseyValleyChainControllerLayout {
	fn default() -> Self {
		Self {
			cell_size: VALLEY_CHAIN_CONTROLLER_CELL_SIZE,
			vertical_half_extent: TERRAIN_CELL_VERTICAL_HALF_EXTENT,
		}
	}
}

impl JerseyValleyChainControllerLayout {
	/// World AABB for controller cell `(ix, iz)`.
	pub fn cell_bounds(&self, ix: i32, iz: i32) -> Aabb3d {
		cell_bounds(ix, iz, self.cell_size, self.vertical_half_extent)
	}
}

/// Bootstrap source used only when first materializing
/// [`JerseyValleyChainControllerLayout`] at [`Id::Universal`].
pub trait BootstrapJerseyValleyChainControllerLayout {
	fn bootstrap_jersey_valley_chain_controller_layout(&self) -> JerseyValleyChainControllerLayout;
}

impl<S> GenerationScheme<S> for JerseyValleyChainControllerLayout
where
	S: BootstrapJerseyValleyChainControllerLayout,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((
			spatial_index.bootstrap_jersey_valley_chain_controller_layout(),
			universal_bounds(),
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

/// Controller-cell [`OriginalId`]s covering `region`.
pub fn original_ids_for_controller_cells<S>(
	spatial_index: &mut S,
	region: Aabb3d,
) -> Vec<OriginalId>
where
	S: GeneratingSpatialIndex<JerseyValleyChainControllerLayout>,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	if GeneratingSpatialIndex::<JerseyValleyChainControllerLayout>::get_or_generate(
		spatial_index,
		Id::Universal,
		&lod_ref,
	)
	.is_none()
	{
		return Vec::new();
	}
	let Some(layout) =
		<S as SpatialIndex<JerseyValleyChainControllerLayout>>::get(spatial_index, Id::Universal)
	else {
		return Vec::new();
	};
	let layout = layout.clone();
	cell_coords_for_region(region, layout.cell_size)
		.map(|(ix, iz)| OriginalId(Id::from_cell(layout.cell_bounds(ix, iz))))
		.filter(|OriginalId(id)| {
			id.origin_cell_bounds().is_some_and(|b| region.intersects(&b))
		})
		.collect()
}
