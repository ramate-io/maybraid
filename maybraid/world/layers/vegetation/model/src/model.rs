//! [`Vegetation`] wrapper and its [`TerrainModel`] impl.

use std::marker::PhantomData;

use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::App;
use lod::lod_ref::LodRef;
use terrain_layer_model::{TerrainCell, TerrainModel};

use crate::vegetation::VegetationModel;

/// Model `V` after vegetation: the same heights as `V::Ground`.
pub struct Vegetation<V>(PhantomData<fn() -> V>);

impl<V> TerrainModel for Vegetation<V>
where
	V: VegetationModel,
{
	type Base = <V::Ground as TerrainModel>::Base;
	type Cell = <V::Ground as TerrainModel>::Cell;
	type Read = <V::Ground as TerrainModel>::Read;
	type Snapshot = <V::Ground as TerrainModel>::Snapshot;
	type Prepare = <V::Ground as TerrainModel>::Prepare;

	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) {
		V::Ground::prepare(prepare, bounds, lod_ref);
	}

	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Option<f32> {
		V::Ground::height_at(read, xz)
	}

	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> f32 {
		V::Ground::fallback_height_at(read, xz)
	}

	fn overlay_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
		target_size: f32,
		overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = <Self::Cell as TerrainCell>::Mesh>> {
		V::Ground::overlay_cell(read, bounds, target_size, overlay_size_tolerance)
	}

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Snapshot {
		V::Ground::snapshot(read, region)
	}

	fn require_generation(app: &App) {
		V::Ground::require_generation(app);
		V::require_generation(app);
	}
}
