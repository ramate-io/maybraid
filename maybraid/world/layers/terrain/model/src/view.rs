//! [`TerrainView`]: the system param layer consumers take.

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;

use crate::model::{TerrainCell, TerrainModel};

/// Read access to model `M`, e.g. `TerrainView<Urbanization<OnTerrain<Durham>>>`.
#[derive(SystemParam)]
pub struct TerrainView<'w, 's, M: TerrainModel> {
	/// Model resources. Layer traits (`UrbanModel`, …) take this borrow.
	pub read: StaticSystemParam<'w, 's, <M as TerrainModel>::Read>,
}

impl<M: TerrainModel> TerrainView<'_, '_, M> {
	/// Stored-only height. `None` means the covering cell is not generated yet.
	pub fn height_at(&self, xz: Vec2) -> Option<f32> {
		M::height_at(&self.read, xz)
	}

	/// Stored height, else the model's analytic fallback.
	pub fn height_or_fallback(&self, xz: Vec2) -> f32 {
		self.height_at(xz).unwrap_or_else(|| M::fallback_height_at(&self.read, xz))
	}

	/// See [`TerrainModel::overlay_cell`].
	pub fn overlay_cell(
		&self,
		bounds: Aabb3d,
		target_size: f32,
		overlay_size_tolerance: Option<f32>,
	) -> Option<&dyn TerrainCell<Mesh = <M::Cell as TerrainCell>::Mesh>> {
		M::overlay_cell(&self.read, bounds, target_size, overlay_size_tolerance)
	}

	pub fn snapshot(&self, region: Aabb3d) -> M::Snapshot {
		M::snapshot(&self.read, region)
	}
}
