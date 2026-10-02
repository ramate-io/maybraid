//! [`UrbanModel`]: the urban artifacts consumers bound on instead of Richmond stores.

use bevy::ecs::system::{ResMut, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use lod::gen::{Id, Version};
use procedural_common::NoiseParams;
use richmond_development_models::{BuiltDevelopment, DevelopmentCell, PadComplex, TerrainWithPads};
use richmond_urbanization::{
	DevelopmentLeaf, UrbanizationExtent, UrbanizationIndex, UrbanizationKind,
};
use durham_terrain_models::TerrainMeshBuilder;
use terrain_layer_model::{TerrainCell, TerrainModel};

use crate::model::Urbanization;
use crate::pads::PadComposable;

/// A terrain model that also carries urbanization: pads, leaves, and developments.
///
/// Read accessors are GET-only over what urbanization generation stored.
/// [`Self::ensure_selected`] is the named write hook mob generation uses today
/// ([#720](https://github.com/ramate-io/maybraid/issues/720)).
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

	/// Built developments overlapping `region`, with the store id and version
	/// host presentation uses.
	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a BuiltDevelopment)>;

	/// Development cell by urbanization leaf id.
	fn development_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a DevelopmentCell>;

	/// Selection noise and pinned kind copied onto the mob index.
	fn urbanization_selection(
		read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (NoiseParams, Option<UrbanizationKind>);

	/// Write access for [`Self::ensure_selected`].
	type Select: SystemParam + 'static;

	/// Cross-layer write kept for [#720](https://github.com/ramate-io/maybraid/issues/720):
	/// mob generation selects urbanization cells over its generate keep.
	fn ensure_selected(select: &mut SystemParamItem<'_, '_, Self::Select>, region: Aabb3d);
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

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a BuiltDevelopment)> {
		read.developments.developments_overlapping_tracked(region)
	}

	fn development_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a DevelopmentCell> {
		read.developments.cell(id)
	}

	fn urbanization_selection(
		read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (NoiseParams, Option<UrbanizationKind>) {
		(read.urbanization.noise, read.urbanization.kind)
	}

	type Select = ResMut<'static, UrbanizationIndex>;

	fn ensure_selected(select: &mut SystemParamItem<'_, '_, Self::Select>, region: Aabb3d) {
		// Today's code passes `mobs.urbanization_noise`, which `sync_mob_models`
		// sets equal to `index.noise` earlier in the same chain, so the result
		// is identical.
		let noise = select.noise;
		for extent in UrbanizationExtent::cells_overlapping(region) {
			select.ensure_selected(extent, noise);
		}
	}
}
