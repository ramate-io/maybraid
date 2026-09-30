//! [`UrbanModel`]: the urban artifacts consumers bound on instead of Richmond stores.

use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use lod::gen::{Id, SpatialIndex, Version};
use richmond_development_models::{BuiltDevelopment, DevelopmentCell, PadComplex, TerrainWithPads};
use richmond_urbanization::{DevelopmentLeaf, SelectedUrbanization};
use durham_terrain_models::TerrainMeshBuilder;
use terrain_layer_model::{TerrainCell, TerrainModel};

use crate::model::Urbanization;
use crate::pads::PadComposable;

/// A terrain model that also carries urbanization: pads, leaves, and developments.
///
/// All accessors are GET-only over what urbanization generation stored.
pub trait UrbanModel: TerrainModel {
	/// Pads merged over `region`.
	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> PadComplex;

	/// Non-empty urbanization guillotine leaves intersecting `region`.
	fn urbanization_leaves<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a DevelopmentLeaf>;

	/// Stored development cells (pads + selection) overlapping `region`.
	fn development_cells<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a DevelopmentCell>;

	/// Built developments (building hosts, places) overlapping `region`.
	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a BuiltDevelopment>;

	/// Urbanization cell ids overlapping `region` (the present-hosts walk).
	fn urbanization_cell_ids(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d)
		-> Vec<Id>;

	/// One stored urbanization cell.
	fn urbanization_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a SelectedUrbanization>;

	/// Development cell by urbanization leaf id.
	fn development_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a DevelopmentCell>;

	/// Built development plus store version, by urbanization leaf id.
	fn built_at<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<(&'a BuiltDevelopment, Version)>;
}

impl<M> UrbanModel for Urbanization<M>
where
	M: TerrainModel,
	M::Cell: PadComposable<Padded = TerrainWithPads> + TerrainCell<Mesh = TerrainMeshBuilder>,
{
	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> PadComplex {
		read.developments.merged_pad_complex(region)
	}

	fn urbanization_leaves<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a DevelopmentLeaf> {
		read.urbanization.filled_leaves_overlapping(region)
	}

	fn development_cells<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a DevelopmentCell> {
		read.developments.filled_cells_overlapping(region)
	}

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a BuiltDevelopment> {
		read.developments.developments_overlapping(region)
	}

	fn urbanization_cell_ids(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<Id> {
		SpatialIndex::<SelectedUrbanization>::tracked_ids_for(&*read.urbanization, region)
			.into_iter()
			.map(|tracked| tracked.0)
			.collect()
	}

	fn urbanization_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a SelectedUrbanization> {
		read.urbanization.get(id)
	}

	fn development_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a DevelopmentCell> {
		read.developments.cell(id)
	}

	fn built_at<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<(&'a BuiltDevelopment, Version)> {
		read.developments.built_at(id)
	}
}
