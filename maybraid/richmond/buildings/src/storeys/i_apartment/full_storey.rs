//! I-Apartment full storey: floor plan plus one [`LivableApartment`] per primary rect.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::LivableApartment;

use super::floor_plan::IApartmentFloorPlan;

/// Full I-Apartment storey: I-frame shell + livable apartments on primary rects.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentFullStorey {
	pub floor_plan: IApartmentFloorPlan,
	pub apartments: Vec<LivableApartment>,
}

impl IApartmentFullStorey {
	/// Wrap an already-fitted floor plan and allocate livable apartments.
	pub fn from_floor_plan(
		floor_plan: IApartmentFloorPlan,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let regions = floor_plan.fillable_regions();
		Self::fill_from_plan(floor_plan, regions, noise)
	}

	fn fill_from_plan(
		floor_plan: IApartmentFloorPlan,
		regions: FillableRegions,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let mut apartments = Vec::new();
		let mut residual_within = Vec::new();

		let _ = noise;
		for (i, region) in regions.within.into_iter().enumerate() {
			match LivableApartment::from_confines(i as u32, &region.confines) {
				Ok((apt, nested)) => {
					apartments.push(apt);
					residual_within.extend(nested.within);
				}
				Err(FitError::TooSmall { .. }) => {
					residual_within.push(region);
				}
				Err(err) => return Err(err),
			}
		}

		Ok((
			Self {
				floor_plan,
				apartments,
			},
			FillableRegions {
				within: residual_within,
				atop: regions.atop,
			},
		))
	}
}

impl Fit for IApartmentFullStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (floor_plan, regions) = IApartmentFloorPlan::fit_to_confines(confines, noise)?;
		Self::fill_from_plan(floor_plan, regions, noise)
	}
}

impl BuildingComponents for IApartmentFullStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		// Outer I-frame from the floor plan; apartments add their region shells.
		let mut out = self.floor_plan.panel_nodes_for_level(level);
		for apt in &self.apartments {
			out.extend(apt.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.floor_plan.joint_nodes_for_level(level);
		for apt in &self.apartments {
			out.extend(apt.joint_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = self.floor_plan.label_nodes_for_level(level);
		for apt in &self.apartments {
			out.extend(apt.label_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use crate::storeys::i_apartment::{IApartmentFloorPlan, IApartmentParameterized};

	#[test]
	fn full_storey_allocates_livable_per_rect() {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		);
		let confines = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = IApartmentParameterized::sample(&confines, noise).unwrap();
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		let n = plan.primary_rects.len();
		let (storey, _) = IApartmentFullStorey::from_floor_plan(plan, noise).unwrap();
		assert_eq!(storey.apartments.len(), n);
		assert!(!storey
			.panel_nodes_for_level(LodSceneLevel::High)
			.is_empty());
	}
}
