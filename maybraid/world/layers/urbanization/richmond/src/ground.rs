//! [`RichmondGround`]: what Richmond reads from a concrete lower model.

use bevy::ecs::system::{ReadOnlySystemParam, Res, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use durham::{
	CellTiling, Durham, HcsgStorage, Terrain, TerrainCellLayout, TerrainMeshBuilder,
	TerrainStorage, Water,
};
use lod::gen::{Id, OriginalId, Version};
use procedural_common::Bounds2;
use terrain_layer_model::{OnTerrain, TerrainCell, TerrainModel};

use crate::compose::PadComposable;
use crate::hydro::hydro_overlaps_xz;
use crate::padded::TerrainWithPads;

/// Ground Richmond generates against. Owned here; implemented for [`OnTerrain<Durham>`].
pub trait RichmondGround:
	TerrainModel<
	Cell: Clone + PadComposable<Padded = TerrainWithPads> + TerrainCell<Mesh = TerrainMeshBuilder>,
>
{
	type GroundRead: ReadOnlySystemParam + 'static;

	fn origin_ids(
		read: &SystemParamItem<'_, '_, Self::GroundRead>,
		region: Aabb3d,
	) -> Vec<OriginalId>;

	fn stored_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::GroundRead>,
		id: Id,
	) -> Option<&'a Self::Cell>;

	fn composed_height_at(
		read: &SystemParamItem<'_, '_, Self::GroundRead>,
		x: f32,
		z: f32,
	) -> Option<f32>;

	fn membership_revision(read: &SystemParamItem<'_, '_, Self::GroundRead>) -> u64;

	fn terrain_ids_overlapping(
		read: &SystemParamItem<'_, '_, Self::GroundRead>,
		region: Aabb3d,
	) -> Vec<Id>;

	fn hydro_overlaps(
		read: &SystemParamItem<'_, '_, Self::GroundRead>,
		cell: Aabb3d,
		bounds: Bounds2,
	) -> bool;

	fn water<'a>(read: &'a SystemParamItem<'_, '_, Self::GroundRead>, id: Id) -> Option<&'a Water>;

	fn water_version(read: &SystemParamItem<'_, '_, Self::GroundRead>, id: Id) -> Option<Version>;
}

/// Durham store + layout. Named only here and in tests.
#[derive(SystemParam)]
pub struct RichmondGroundView<'w> {
	store: Res<'w, HcsgStorage>,
	layout: Res<'w, TerrainCellLayout>,
}

impl RichmondGround for OnTerrain<Durham> {
	type GroundRead = RichmondGroundView<'static>;

	fn origin_ids(
		read: &SystemParamItem<'_, '_, Self::GroundRead>,
		region: Aabb3d,
	) -> Vec<OriginalId> {
		read.layout.cell_ids(region)
	}

	fn stored_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::GroundRead>,
		id: Id,
	) -> Option<&'a Terrain> {
		read.store.terrain(id)
	}

	fn composed_height_at(
		read: &SystemParamItem<'_, '_, Self::GroundRead>,
		x: f32,
		z: f32,
	) -> Option<f32> {
		read.store.composed_height_at(&read.layout, x, z)
	}

	fn membership_revision(read: &SystemParamItem<'_, '_, Self::GroundRead>) -> u64 {
		read.store.terrain_revision()
	}

	fn terrain_ids_overlapping(
		read: &SystemParamItem<'_, '_, Self::GroundRead>,
		region: Aabb3d,
	) -> Vec<Id> {
		read.store.terrain_ids_overlapping(region)
	}

	fn hydro_overlaps(
		read: &SystemParamItem<'_, '_, Self::GroundRead>,
		cell: Aabb3d,
		bounds: Bounds2,
	) -> bool {
		for OriginalId(id) in Self::origin_ids(read, cell) {
			let Some(terrain) = read.store.terrain(id) else {
				continue;
			};
			if hydro_overlaps_xz(&terrain.marazion_fills, bounds) {
				return true;
			}
		}
		false
	}

	fn water<'a>(read: &'a SystemParamItem<'_, '_, Self::GroundRead>, id: Id) -> Option<&'a Water> {
		read.store.water(id)
	}

	fn water_version(read: &SystemParamItem<'_, '_, Self::GroundRead>, id: Id) -> Option<Version> {
		read.store.water_version(id)
	}
}
