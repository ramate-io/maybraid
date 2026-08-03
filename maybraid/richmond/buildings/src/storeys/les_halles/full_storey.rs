//! Les Halles full storey: floor plan plus commercial gallery strip fills.
//!
//! The ring shell lives on [`LesHallesFloorPlan`]. This type fits that plan and
//! fills [`SpaceKind::ExternalSpace`] strips with [`CommercialStallStrip`].
//! Residual walkways / shafts remain in [`FillableRegions::within`].

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError, SpaceKind};
use crate::usage_areas::CommercialStallStrip;

use super::floor_plan::LesHallesFloorPlan;

/// Full Les Halles storey: shell on [`Self::floor_plan`] plus gallery stall strips.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesFullStorey {
	pub floor_plan: LesHallesFloorPlan,
	pub stall_strips: Vec<CommercialStallStrip>,
}

impl LesHallesFullStorey {
	/// Wrap an already-fitted floor plan and fill external gallery strips.
	pub fn from_floor_plan(
		floor_plan: LesHallesFloorPlan,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let regions = floor_plan.fillable_regions();
		Self::fill_from_regions(floor_plan, regions, noise)
	}

	fn fill_from_regions(
		floor_plan: LesHallesFloorPlan,
		regions: FillableRegions,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let mut stall_strips = Vec::new();
		let mut residual_within = Vec::new();
		for (i, region) in regions.within.into_iter().enumerate() {
			if region.kind != SpaceKind::ExternalSpace {
				residual_within.push(region);
				continue;
			}
			let mut strip_noise = noise;
			strip_noise.seed = noise.seed.wrapping_add(i as i32 * 31);
			match CommercialStallStrip::fit_to_confines(&region.confines, strip_noise) {
				Ok((strip, _)) => stall_strips.push(strip),
				Err(FitError::TooSmall { .. }) => {
					// Leave unfilled if the strip is too narrow after shaft clears.
					residual_within.push(region);
				}
				Err(err) => return Err(err),
			}
		}
		Ok((
			Self {
				floor_plan,
				stall_strips,
			},
			FillableRegions {
				within: residual_within,
				atop: regions.atop,
			},
		))
	}
}

impl Fit for LesHallesFullStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (floor_plan, regions) = LesHallesFloorPlan::fit_to_confines(confines, noise)?;
		Self::fill_from_regions(floor_plan, regions, noise)
	}
}

impl BuildingComponents for LesHallesFullStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.floor_plan.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.floor_plan.joint_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for strip in &self.stall_strips {
			out.extend(strip.label_nodes_for_level(level));
		}
		out
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
	fn full_storey_fills_external_strips_with_labels() {
		use crate::storeys::les_halles::{LesHallesFloorPlan, LesHallesParameterized};

		let bounds = Aabb3d::from_min_max(
			Vec3::new(-24.0, 0.0, -18.0),
			Vec3::new(24.0, 4.0, 18.0),
		);
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = LesHallesParameterized::sample(&empty, noise).unwrap();
		let openings = LesHallesFloorPlan::shaft_requests_for_all_slots(&params, &empty);
		let confines = Confines::new(bounds, 0.0, openings);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(params, &confines).unwrap();
		let (storey, regions) = LesHallesFullStorey::from_floor_plan(plan, noise).unwrap();
		assert!(!storey.stall_strips.is_empty());
		assert!(regions
			.within
			.iter()
			.all(|r| r.kind != SpaceKind::ExternalSpace));
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::Walkway));
		assert_eq!(regions.atop.len(), 1);
		assert!(storey.floor_plan.gallery.wall_count() >= 4);
		assert!(!storey
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
		let panels = storey.panel_nodes_for_level(LodSceneLevel::High);
		assert!(!panels.is_empty());
	}
}
