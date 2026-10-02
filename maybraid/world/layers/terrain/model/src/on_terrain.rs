//! [`OnTerrain`]: the ground surface of a terrain model.

use std::marker::PhantomData;

use bevy::app::App;
use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use lod::gen::Id;
use lod::lod_ref::LodRef;

use crate::model::{TerrainCell, TerrainModel};

/// The solid-ground surface of model `T`, as opposed to its other outputs (water).
///
/// Transparent today. General ground logic that every layer on top of a terrain
/// should share belongs here rather than in each consumer.
pub struct OnTerrain<T>(PhantomData<fn() -> T>);

impl<T: TerrainModel> TerrainModel for OnTerrain<T> {
	type Cell = T::Cell;
	type Read = T::Read;
	type Snapshot = T::Snapshot;
	type Prepare = T::Prepare;

	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) {
		T::prepare(prepare, bounds, lod_ref);
	}

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

	fn overlay_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
		target_size: f32,
		overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = <Self::Cell as TerrainCell>::Mesh>> {
		T::overlay_cell(read, bounds, target_size, overlay_size_tolerance)
	}

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Snapshot {
		T::snapshot(read, region)
	}

	fn require_generation(app: &App) {
		T::require_generation(app);
	}
}
