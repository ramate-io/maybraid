//! [`OnTerrain`]: the ground surface of a terrain model.

use std::marker::PhantomData;

use bevy::app::App;
use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use lod::gen::Id;

use crate::model::TerrainModel;

/// The solid-ground surface of model `T`, as opposed to its other outputs (water).
///
/// Transparent today. General ground logic that every layer on top of a terrain
/// should share belongs here rather than in each consumer.
pub struct OnTerrain<T>(PhantomData<fn() -> T>);

impl<T: TerrainModel> TerrainModel for OnTerrain<T> {
	type Cell = T::Cell;
	type Read = T::Read;
	type Snapshot = T::Snapshot;

	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Option<f32> {
		T::height_at(read, xz)
	}

	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> f32 {
		T::fallback_height_at(read, xz)
	}

	fn cell_ids_overlapping(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Vec<Id> {
		T::cell_ids_overlapping(read, region)
	}

	fn cell<'a>(read: &'a SystemParamItem<'_, '_, Self::Read>, id: Id) -> Option<&'a Self::Cell> {
		T::cell(read, id)
	}

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Snapshot {
		T::snapshot(read, region)
	}

	fn require_generation(app: &App) {
		T::require_generation(app);
	}
}
