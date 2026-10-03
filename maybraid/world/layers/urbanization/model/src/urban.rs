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

	type Built: Send + Sync + 'static;

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Built>;

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a Self::Built)>;

	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	);

	fn require_generation(app: &App);
}

/// Urbanized ground: pads and the built developments stored on it.
pub trait UrbanModel: TerrainModel {
	type Built: Send + Sync + 'static;
	type Pads: PadOps;

	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Pads;

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a Self::Built>;

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a Self::Built)>;
}
