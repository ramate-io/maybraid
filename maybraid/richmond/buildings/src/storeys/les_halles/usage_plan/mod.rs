//! Usage plans that paint onto [`LesHallesFloorPlan`] residuals.
//!
//! The floor plan owns the shell. [`FillableRegions`] (from
//! [`LesHallesFloorPlan::fillable_regions`]) carries the typed residuals —
//! especially [`SpaceKind::ExternalSpace`] gallery strips. A usage plan consumes
//! those regions and returns presentable fill plus leftovers (walkways, shafts, …).
//!
//! Full\* storeys and monotowers keep the plan separately and call [`LesHallesUsagePlan::paint`].

mod livable;

pub use livable::LesHallesLivableUsage;

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{FillableRegions, Fit, FitError, SpaceKind};
use crate::usage_areas::CommercialStallStrip;

/// Paint Les Halles gallery residuals into a presentable usage layer.
pub trait LesHallesUsagePlan: Sized {
	fn paint(
		regions: FillableRegions,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError>;
}

/// Commercial gallery fill: one [`CommercialStallStrip`] per ExternalSpace strip.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesCommercialUsage {
	pub stall_strips: Vec<CommercialStallStrip>,
}

impl LesHallesUsagePlan for LesHallesCommercialUsage {
	fn paint(
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
					residual_within.push(region);
				}
				Err(err) => return Err(err),
			}
		}
		Ok((
			Self { stall_strips },
			FillableRegions {
				within: residual_within,
				atop: regions.atop,
			},
		))
	}
}

impl BuildingComponents for LesHallesCommercialUsage {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for strip in &self.stall_strips {
			out.extend(strip.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<JointNode> {
		Layers::new()
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for strip in &self.stall_strips {
			out.extend(strip.label_nodes_for_level(level));
		}
		out
	}
}
