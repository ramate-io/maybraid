//! Les Halles full storey: floor plan plus (future) child typology fills.
//!
//! The ring shell lives on [`LesHallesFloorPlan`]. This type fits that plan and
//! leaves residual [`FillableRegions::within`] for shops / stairs / furniture.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::floor_plan::LesHallesFloorPlan;

/// Full Les Halles storey. Shell geometry is on [`Self::floor_plan`]; child fills
/// are not authored in this slice and remain in residual `within` regions.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesFullStorey {
	pub floor_plan: LesHallesFloorPlan,
}

impl LesHallesFullStorey {
	/// Wrap an already-fitted floor plan (towering / reuse path).
	pub fn from_floor_plan(floor_plan: LesHallesFloorPlan) -> (Self, FillableRegions) {
		let regions = floor_plan.fillable_regions();
		(Self { floor_plan }, regions)
	}
}

impl Fit for LesHallesFullStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (floor_plan, regions) = LesHallesFloorPlan::fit_to_confines(confines, noise)?;
		Ok((Self { floor_plan }, regions))
	}
}

impl BuildingComponents for LesHallesFullStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.floor_plan.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.floor_plan.joint_nodes_for_level(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use lod::gen::LodSceneLevel;
	use procedural_common::NoiseParams;
	use richmond_building_components::BuildingComponents;

	#[test]
	fn full_storey_fit_emits_shell_panels() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-10.0, 0.0, -8.0),
			Vec3::new(10.0, 3.5, 8.0),
		));
		let (storey, regions) =
			LesHallesFullStorey::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(!regions.within.is_empty());
		assert_eq!(regions.atop.len(), 1);
		assert!(storey.floor_plan.shell.wall_count() >= 4);
		assert!(storey.floor_plan.shell.has_floor());
		let panels = storey.panel_nodes_for_level(LodSceneLevel::High);
		assert!(!panels.is_empty());
	}
}
