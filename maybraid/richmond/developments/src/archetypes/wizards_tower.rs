use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use material_ref::MaterialRef;
use procedural_common::NoiseParams;
use richmond_buildings::wizards_tower::WizardsTower;
use richmond_buildings::{CellConstraints, Confines, FillableRegions, Fit, FitError};

use crate::BuildingFootprint;

/// Solitary integration wrapper for the existing procedural Wizard's Tower.
#[derive(Debug, Clone, PartialEq)]
pub struct SolitaryWizardsTower {
	pub bounds: Aabb3d,
	pub tower: WizardsTower,
}

impl SolitaryWizardsTower {
	pub fn with_finish(mut self, wall: MaterialRef, room: MaterialRef) -> Self {
		self.tower = self.tower.with_finish(wall, room);
		self
	}
}

impl Fit for SolitaryWizardsTower {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let footprint = confines.footprint();
		let side = footprint.x.min(footprint.y) * 0.72;
		let height = confines.bounds.max.y - confines.bounds.min.y;
		let storey_height = 4.0;
		let available_floors = (height / storey_height).floor() as u32;
		let available_floors = available_floors.saturating_sub(1);
		if side < 12.0 || available_floors < 10 {
			return Err(FitError::TooSmall { reason: "solitary_wizards_tower" });
		}
		let floor_count = available_floors.min(30);
		let center = confines.center();
		let half = side * 0.5;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - half, confines.bounds.min.y, center.z - half),
			Vec3::new(
				center.x + half,
				confines.bounds.min.y + (floor_count + 1) as f32 * storey_height,
				center.z + half,
			),
		);
		let constraints = CellConstraints::cell_owned(bounds);
		let floor_noise = (floor_count - 10) as f32 / 20.0;
		let tower = WizardsTower::new(&constraints, floor_noise);
		Ok((Self { bounds, tower }, FillableRegions { within: Vec::new(), atop: Vec::new() }))
	}
}

impl BuildingFootprint for SolitaryWizardsTower {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![Aabb2d {
			min: Vec2::new(self.bounds.min.x, self.bounds.min.z),
			max: Vec2::new(self.bounds.max.x, self.bounds.max.z),
		}]
	}
}

delegate_components!(SolitaryWizardsTower, tower);
