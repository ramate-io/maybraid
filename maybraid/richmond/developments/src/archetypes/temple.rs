use bevy_math::bounding::Aabb3d;
use material_ref::MaterialRef;

use crate::{PlacedBuilding, ShepherdsVillageBuilding, TempleSanctum};

/// Axial halls around a taller central shrine.
#[derive(Debug, Clone, PartialEq)]
pub struct TempleComplex {
	pub bounds: Aabb3d,
	pub halls: Vec<ShepherdsVillageBuilding>,
	pub sanctum: PlacedBuilding<TempleSanctum>,
}

impl TempleComplex {
	pub fn with_finish(mut self, wall: MaterialRef, ornament: MaterialRef) -> Self {
		self.sanctum.building = self.sanctum.building.with_finish(wall, ornament);
		self
	}

	pub fn with_landmark_material(mut self, wall: MaterialRef) -> Self {
		self.sanctum.building = self.sanctum.building.with_wall_material(wall);
		self
	}
}
