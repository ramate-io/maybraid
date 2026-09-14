//! Usage plans that paint onto [`LesHallesFloorPlan`] residuals.
//!
//! The floor plan owns the shell. [`FillableRegions`] (from
//! [`LesHallesFloorPlan::fillable_regions`]) carries the typed residuals —
//! especially [`SpaceKind::ExternalSpace`] gallery strips. A usage plan consumes
//! those regions and returns presentable fill plus leftovers (walkways, shafts, …).
//!
//! Full\* storeys and [`crate::monotower`] stacks keep the plan separately and
//! call [`LesHallesUsagePlan::paint`].

mod livable;

pub use livable::LesHallesLivableUsage;

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{FillableRegions, Fit, FitError, SpaceKind};
use crate::usage_areas::furniture_util::{
	as_closet_if_internal, chest_in_confines, chests_for_regions, FurnitureFill,
};
use crate::usage_areas::{CommercialStallInterior, CommercialStallStrip};

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
	/// Chests in leftover gallery / closet pockets the stall packer could not fill.
	pub residual_chests: Vec<FurnitureFill>,
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
				Ok((strip, leftover)) => {
					stall_strips.push(strip);
					residual_within.extend(leftover.within.into_iter().map(as_closet_if_internal));
				}
				Err(FitError::TooSmall { .. }) => {
					residual_within.push(region);
				}
				Err(err) => return Err(err),
			}
		}
		let mut residual_chests = chests_for_regions(&residual_within, noise);
		for (si, strip) in stall_strips.iter().enumerate() {
			for (ti, stall) in strip.stalls().iter().enumerate() {
				if !matches!(stall.interior(), CommercialStallInterior::Lounge(_)) {
					continue;
				}
				if let Some(fill) =
					chest_in_confines(&stall.confines, noise, 300 + si as u32 * 17 + ti as u32)
				{
					residual_chests.push(fill);
				}
			}
		}
		Ok((
			Self { stall_strips, residual_chests },
			FillableRegions { within: residual_within, atop: regions.atop },
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

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::from_free(self.residual_chests.iter().map(|fill| fill.furniture.clone()).collect())
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for strip in &self.stall_strips {
			out.extend(strip.label_nodes_for_level(level));
		}
		out.extend(Layers::from_free(
			self.residual_chests.iter().map(|fill| fill.label.clone()).collect(),
		));
		out
	}
}

/// Ground arcade: gallery strips stay open (no stall / apartment fill).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LesHallesArcadeUsage;

impl LesHallesArcadeUsage {
	pub fn is_empty(&self) -> bool {
		true
	}
}

impl LesHallesUsagePlan for LesHallesArcadeUsage {
	fn paint(
		regions: FillableRegions,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Ok((Self, regions))
	}
}
