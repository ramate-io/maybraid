//! I-Apartment full storey: floor plan → [`LivableApartments`] per primary rect.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::{LivableApartments, LivableApartmentsOptions};

use super::floor_plan::IApartmentFloorPlan;

/// Full I-Apartment storey: I-frame shell + livable apartment blocks per primary rect.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentFullStorey {
	pub floor_plan: IApartmentFloorPlan,
	/// One [`LivableApartments`] pack per primary rectangular residual.
	pub blocks: Vec<LivableApartments>,
}

impl IApartmentFullStorey {
	/// Wrap an already-fitted floor plan and allocate livable apartment blocks.
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
		let mut blocks = Vec::new();
		let mut residual_within = Vec::new();
		let opts = LivableApartmentsOptions {
			hall_width: Some(floor_plan.hall_width),
		};

		for region in regions.within {
			match LivableApartments::from_confines_with(&region.confines, noise, opts) {
				Ok((block, nested)) => {
					blocks.push(block);
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
				blocks,
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
		let mut out = self.floor_plan.panel_nodes_for_level(level);
		for block in &self.blocks {
			out.extend(block.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.floor_plan.joint_nodes_for_level(level);
		for block in &self.blocks {
			out.extend(block.joint_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = self.floor_plan.label_nodes_for_level(level);
		for block in &self.blocks {
			out.extend(block.label_nodes_for_level(level));
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
	fn full_storey_allocates_block_per_rect() {
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
		assert_eq!(storey.blocks.len(), n);
		assert!(!storey
			.panel_nodes_for_level(LodSceneLevel::High)
			.is_empty());
	}

	#[test]
	fn full_storey_blocks_contain_apartments_when_connected() {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		);
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = IApartmentParameterized::sample(&empty, noise).unwrap();
		let inbound = IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty);
		let confines = Confines::new(bounds, 0.0, inbound);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		let (storey, _) = IApartmentFullStorey::from_floor_plan(plan, noise).unwrap();
		assert!(!storey.blocks.is_empty());
		// Inter-rect passages and/or shafts give HallsToShafts work on multi-rect plans;
		// at least one block should produce apartments or retain hallway residuals.
		assert!(
			storey.blocks.iter().any(|b| !b.apartments.is_empty())
				|| storey.blocks.iter().any(|b| !b.halls.hall_bands.is_empty()),
			"expected apartments or carved halls in at least one block"
		);
	}
}
