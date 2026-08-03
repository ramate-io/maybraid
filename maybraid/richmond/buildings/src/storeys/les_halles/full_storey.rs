//! Les Halles full storey: floor plan + rectangular ring shell.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::shells::rect_ring_floor::{RectRingFloor, RectRingFloorParams, RectRingFloorSlab};

use super::floor_plan::LesHallesFloorPlan;

/// Shell-backed Les Halles storey. Child typology fills remain in residual
/// [`FillableRegions::within`] for a later pass.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesFullStorey {
	pub floor_plan: LesHallesFloorPlan,
	pub shell: RectRingFloor,
}

impl LesHallesFullStorey {
	/// Build a shell-backed storey from an already-fitted floor plan.
	pub fn from_floor_plan(floor_plan: LesHallesFloorPlan) -> (Self, FillableRegions) {
		let regions = floor_plan.fillable_regions();
		let shell = build_shell(&floor_plan);
		(Self { floor_plan, shell }, regions)
	}
}

impl Fit for LesHallesFullStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (floor_plan, regions) = LesHallesFloorPlan::fit_to_confines(confines, noise)?;
		let shell = build_shell(&floor_plan);
		Ok((Self { floor_plan, shell }, regions))
	}
}

impl BuildingComponents for LesHallesFullStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.shell.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.shell.joint_nodes_for_level(level)
	}
}

fn build_shell(floor_plan: &LesHallesFloorPlan) -> RectRingFloor {
	RectRingFloor::new(
		RectRingFloorParams::new(
			floor_plan.center_xz,
			floor_plan.outer,
			floor_plan.inner,
			floor_plan.storey_height,
		)
		.floor(RectRingFloorSlab::Solid)
		.ceiling(RectRingFloorSlab::Solid)
		.openings(floor_plan.openings.clone()),
	)
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
		assert!(storey.shell.wall_count() > 0);
		assert!(storey.shell.has_floor());
		let panels = storey.panel_nodes_for_level(LodSceneLevel::High);
		assert!(!panels.is_empty());
		assert!(storey.shell.wall_count() >= 4);
	}
}
