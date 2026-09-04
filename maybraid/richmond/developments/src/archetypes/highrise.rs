use bevy_math::bounding::Aabb2d;

use crate::BuildingFootprint;

pub use richmond_buildings::{
	ApartmentMonotower, SingleHighrise, SingleHighriseFloorPlan, SingleHighrisePlan,
	SingleHighriseShaftSlot, SingleHighriseStorey,
};

impl BuildingFootprint for SingleHighrise {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![self.tower.floor_plan.footprint_bounds()]
	}
}
