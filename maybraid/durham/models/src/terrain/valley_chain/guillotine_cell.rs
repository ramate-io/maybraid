//! Irregular guillotine leaf identities transferred into the spatial index.

use crate::terrain::valley_chain::controller::JerseyValleyChainControllerCell;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;

/// One guillotine leaf under a ValleyChain controller.
#[derive(Debug, Clone, Component)]
pub struct JerseyValleyChainGuillotineCell {
	pub cell: Aabb3d,
	pub controller_id: Id,
	pub leaf_index: u32,
}

/// Discover leaf [`OriginalId`]s by materializing overlapping controllers and
/// enumerating their guillotine regions.
pub fn original_ids_for_guillotine_leaves<S>(
	spatial_index: &mut S,
	region: Aabb3d,
) -> Vec<OriginalId>
where
	S: GeneratingSpatialIndex<JerseyValleyChainControllerCell>,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	let controllers =
		GeneratingSpatialIndex::<JerseyValleyChainControllerCell>::get_or_generate_region(
			spatial_index,
			region,
			&lod_ref,
		);
	let mut out = Vec::new();
	for (controller_id, _) in controllers {
		let Some(controller) =
			<S as SpatialIndex<JerseyValleyChainControllerCell>>::get(spatial_index, controller_id)
		else {
			continue;
		};
		for leaf in controller.leaf_aabbs() {
			if region.intersects(&leaf) {
				out.push(OriginalId(Id::from_cell(leaf)));
			}
		}
	}
	out.sort();
	out.dedup();
	out
}

impl<S> GenerationScheme<S> for JerseyValleyChainGuillotineCell
where
	S: GeneratingSpatialIndex<JerseyValleyChainControllerCell>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_guillotine_leaves(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let controllers =
			GeneratingSpatialIndex::<JerseyValleyChainControllerCell>::get_or_generate_region(
				spatial_index,
				bounds,
				lod_ref,
			);
		for (controller_id, _) in controllers {
			let Some(controller) = <S as SpatialIndex<JerseyValleyChainControllerCell>>::get(
				spatial_index,
				controller_id,
			) else {
				continue;
			};
			for (leaf_index, leaf) in controller.leaf_aabbs().into_iter().enumerate() {
				if Id::from_cell(leaf) == id {
					return Some((
						Self {
							cell: bounds,
							controller_id,
							leaf_index: leaf_index as u32,
						},
						bounds,
					));
				}
			}
		}
		None
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
