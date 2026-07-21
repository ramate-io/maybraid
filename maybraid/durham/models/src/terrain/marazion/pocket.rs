//! Pocket cells with RFC-style guillotine leaves ([RFC-127 §3.1.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#312-pocket-cells)).

use crate::terrain::jersey::shared::LeafAabbs;
use crate::terrain::marazion::config::MarazionWatershedConfigs;
use crate::terrain::marazion::pre_pocket::{aabb_from_bounds2, pocket_aabb, PrePocketCell};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use marazion_watersheds::guillotine_partition;
use procedural_common::Bounds2;

/// One pocket tile + guillotine leaf AABBs.
#[derive(Debug, Clone, Component)]
pub struct PocketCell {
	pub cell: Aabb3d,
	pub leaves: Vec<Aabb3d>,
}

impl LeafAabbs for PocketCell {
	fn leaf_aabbs(&self) -> Vec<Aabb3d> {
		self.leaves.clone()
	}
}

pub fn original_ids_for_pocket_cells<S>(
	spatial_index: &mut S,
	region: Aabb3d,
) -> Vec<OriginalId>
where
	S: GeneratingSpatialIndex<PrePocketCell>,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	let pre_cells = GeneratingSpatialIndex::<PrePocketCell>::get_or_generate_region(
		spatial_index,
		region,
		&lod_ref,
	);
	let mut out = Vec::new();
	for (pre_id, _) in pre_cells {
		let Some(pre_cell) = <S as SpatialIndex<PrePocketCell>>::get(spatial_index, pre_id) else {
			continue;
		};
		let vy_min = pre_cell.cell.min.y;
		let vy_max = pre_cell.cell.max.y;
		for px in 0..pre_cell.pre.nx {
			for pz in 0..pre_cell.pre.nz {
				let aabb = pocket_aabb(&pre_cell.pre, px, pz, vy_min, vy_max);
				if region.intersects(&aabb) {
					out.push(OriginalId(Id::from_cell(aabb)));
				}
			}
		}
	}
	out.sort_by(|a, b| a.0.cmp(&b.0));
	out.dedup();
	out
}

impl<S> GenerationScheme<S> for PocketCell
where
	S: GeneratingSpatialIndex<MarazionWatershedConfigs> + GeneratingSpatialIndex<PrePocketCell>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_pocket_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let cell = id.origin_cell_bounds()?;
		let configs = GeneratingSpatialIndex::<MarazionWatershedConfigs>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let mut gparams = configs.guillotine;
		gparams.seed = configs.seed.wrapping_add(0x6011);
		let bounds = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
		let leaves: Vec<Aabb3d> = guillotine_partition(bounds, &gparams)
			.into_iter()
			.map(|b| aabb_from_bounds2(b, cell.min.y, cell.max.y))
			.collect();
		Some((Self { cell, leaves }, cell))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
