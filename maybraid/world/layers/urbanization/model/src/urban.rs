//! [`UrbanizationModel`] and the consumer-facing [`UrbanModel`] bound.

use bevy::app::App;
use bevy::ecs::system::{ReadOnlySystemParam, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use terrain_layer_model::{TerrainCell, TerrainModel};

use crate::pads::PadOps;

/// Named urbanization over a ground model.
///
/// Only what [`crate::Urbanization<Self>`]'s [`TerrainModel`] impl and
/// urbanization presentation read. No method exists for a higher layer.
pub trait UrbanizationModel: Send + Sync + 'static {
	type Ground: TerrainModel;
	type Pads: PadOps + Clone + Send + Sync + 'static;
	/// Composed fill stored as [`crate::Urbanization<Self>`]'s [`TerrainModel::Cell`].
	type Surface: TerrainCell;
	type Read: ReadOnlySystemParam + 'static;
	/// Present-time prepare [`crate::Urbanization<Self>`] forwards as [`TerrainModel::Prepare`].
	type Prepare: SystemParam + 'static;

	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Pads;

	fn pads_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Self::Pads;

	fn surface<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a Self::Surface>;

	fn surface_ids(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Vec<Id>;

	fn overlay_surface<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
	) -> Option<&'a Self::Surface>;

	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	);

	fn require_generation(app: &App);
}

/// Artifacts Richmond stores; [`UrbanModel`] on [`crate::Urbanization<Self>`] forwards these.
///
/// Orphan rules keep the [`UrbanModel`] impl in this crate. Richmond implements this.
pub trait UrbanSource: UrbanizationModel {
	type Leaf: Send + Sync + 'static;
	type Cell: Send + Sync + 'static;
	type Built: Send + Sync + 'static;
	type Kind: Copy + Send + Sync + 'static;
	type Selection: Clone + PartialEq + Send + Sync + 'static;
	type Select: SystemParam + 'static;

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Built>;

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a Self::Built)>;

	fn leaves<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Leaf>;

	fn cells<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Cell>;

	fn selection(
		read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (Self::Selection, Option<Self::Kind>);

	fn leaf_aabb(leaf: &Self::Leaf) -> Aabb3d;

	fn cell_aabb(cell: &Self::Cell) -> Aabb3d;

	fn select(select: &mut SystemParamItem<'_, '_, Self::Select>, region: Aabb3d);
}

/// Urbanized ground: pads and built developments, plus Barking's current reads.
///
/// [`Self::urbanization_selection`], [`Self::ensure_selected`] / [`Self::Select`],
/// [`Self::urbanization_leaves`], [`Self::development_cells`], [`Self::leaf_bounds`],
/// [`Self::cell_bounds`], [`Self::Selection`], and [`Self::Kind`] are mob-only.
/// [#925](https://github.com/ramate-io/maybraid/issues/925) moves them into Barking-owned traits.
pub trait UrbanModel: TerrainModel {
	type Leaf: Send + Sync + 'static;
	type Cell: Send + Sync + 'static;
	type Built: Send + Sync + 'static;
	type Pads: PadOps;
	type Kind: Copy + Send + Sync + 'static;
	type Selection: Clone + PartialEq + Send + Sync + 'static;
	type Select: SystemParam + 'static;

	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Pads;

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Built>;

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a Self::Built)>;

	fn urbanization_leaves<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Leaf>;

	fn development_cells<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a <Self as UrbanModel>::Cell>;

	fn urbanization_selection(
		read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (Self::Selection, Option<Self::Kind>);

	fn leaf_bounds(leaf: &Self::Leaf) -> Aabb3d;

	fn cell_bounds(cell: &<Self as UrbanModel>::Cell) -> Aabb3d;

	fn ensure_selected(select: &mut SystemParamItem<'_, '_, Self::Select>, region: Aabb3d);
}
