//! Ground-floor arcade: open gallery ring with midspan breezeways, no stall fill.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::floor_plan::{LesHallesFloorPlan, LesHallesOpeningProgram};
use super::parameterized::LesHallesParameterized;
use super::usage_plan::{LesHallesArcadeUsage, LesHallesUsagePlan};

/// Full Les Halles ground storey: ring shell plus empty arcade usage.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesArcadeStorey {
	pub floor_plan: LesHallesFloorPlan,
	pub usage: LesHallesArcadeUsage,
}

impl LesHallesArcadeStorey {
	/// Wrap an already-fitted arcade floor plan.
	pub fn from_floor_plan(
		floor_plan: LesHallesFloorPlan,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let regions = floor_plan.fillable_regions();
		let (usage, residual) = LesHallesArcadeUsage::paint(regions, noise)?;
		Ok((Self { floor_plan, usage }, residual))
	}
}

impl Fit for LesHallesArcadeStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = LesHallesParameterized::sample_monotower(confines, noise)
			.or_else(|_| LesHallesParameterized::sample(confines, noise))?;
		let (floor_plan, regions) = LesHallesFloorPlan::from_parameterized_with(
			params,
			confines,
			crate::shells::rect_ring_floor::RectRingFloorSlab::None,
			LesHallesOpeningProgram::GroundArcade,
		)?;
		let (usage, residual) = LesHallesArcadeUsage::paint(regions, noise)?;
		Ok((Self { floor_plan, usage }, residual))
	}
}

impl BuildingComponents for LesHallesArcadeStorey {
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
	use procedural_common::NoiseParams;

	use crate::fit::SpaceKind;
	use crate::openings::OpeningLabel;

	#[test]
	fn arcade_storey_leaves_gallery_unfilled() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-36.0, 0.0, -27.0),
			Vec3::new(36.0, 4.0, 27.0),
		));
		let (storey, residual) =
			LesHallesArcadeStorey::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(storey.floor_plan.openings.iter().any(|(id, o)| {
			id.as_str().contains("outer_breezeway") && matches!(o.label, OpeningLabel::Passage)
		}));
		assert!(residual.within.iter().any(|r| r.kind == SpaceKind::ExternalSpace));
		assert!(storey.usage.is_empty());
	}
}
