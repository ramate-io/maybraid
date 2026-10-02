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
/// Associated types are the artifacts current consumers read. Methods are the
/// GET-only accessors those consumers call plus the writes [`UrbanModel`]
/// already exposed (`ensure_selected`, `#720` prepare).
pub trait UrbanizationModel: Send + Sync + 'static {
	type Ground: TerrainModel;
	type Leaf: Send + Sync + 'static;
	type Cell: Send + Sync + 'static;
	type Built: Send + Sync + 'static;
	type Pads: PadOps + Clone + Send + Sync + 'static;
	type Kind: Copy + Send + Sync + 'static;
	type Selection: Clone + PartialEq + Send + Sync + 'static;
	/// Composed fill stored as [`crate::Urbanization<Self>`]'s [`TerrainModel::Cell`].
	type Surface: TerrainCell;
	type Read: ReadOnlySystemParam + 'static;
	type Select: SystemParam + 'static;
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

	fn urbanization_leaves<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Leaf>;

	fn development_cells<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Cell>;

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Built>;

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a Self::Built)>;

	fn development_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a Self::Cell>;

	fn urbanization_selection(
		read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (Self::Selection, Option<Self::Kind>);

	fn leaf_bounds(leaf: &Self::Leaf) -> Aabb3d;

	fn cell_bounds(cell: &Self::Cell) -> Aabb3d;

	fn ensure_selected(select: &mut SystemParamItem<'_, '_, Self::Select>, region: Aabb3d);

	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	);

	fn require_generation(app: &App);
}

/// Terrain model that also carries urbanization artifacts.
///
/// Read accessors are GET-only. [`Self::ensure_selected`] is the named write
/// hook mob generation uses today
/// ([#720](https://github.com/ramate-io/maybraid/issues/720)).
pub trait UrbanModel: TerrainModel {
	type Leaf: Send + Sync + 'static;
	type Cell: Send + Sync + 'static;
	type Built: Send + Sync + 'static;
	type Pads: PadOps;
	type Kind: Copy + Send + Sync + 'static;
	type Selection: Clone + PartialEq + Send + Sync + 'static;
	type Select: SystemParam + 'static;

	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Pads;

	fn urbanization_leaves<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Leaf>;

	fn development_cells<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a <Self as UrbanModel>::Cell>;

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Built>;

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a Self::Built)>;

	fn development_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a <Self as UrbanModel>::Cell>;

	fn urbanization_selection(
		read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (Self::Selection, Option<Self::Kind>);

	fn leaf_bounds(leaf: &Self::Leaf) -> Aabb3d;

	fn cell_bounds(cell: &<Self as UrbanModel>::Cell) -> Aabb3d;

	fn ensure_selected(select: &mut SystemParamItem<'_, '_, Self::Select>, region: Aabb3d);
}
