//! Single commercial stall usage area (Label placeholder).

use bevy_math::bounding::BoundingVolume;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};

/// Noise knobs for a commercial stall placeholder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommercialStallParameterized {
	pub style: LabelStyle,
}

impl CommercialStallParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Self {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 11.0);
		Self {
			style: LabelStyle::from_unit(t),
		}
	}
}

/// Stall plan: one labeled volume filling the confines.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStallPlan {
	pub parameterized: CommercialStallParameterized,
	pub label: LabelNode,
}

impl CommercialStallPlan {
	pub fn from_parameterized(
		params: CommercialStallParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let min = Vec3::from(confines.bounds.min);
		let max = Vec3::from(confines.bounds.max);
		let extent = (max - min).max(Vec3::splat(1e-4));
		if extent.x < 0.4 || extent.z < 0.4 || extent.y < 0.4 {
			return Err(FitError::TooSmall { reason: "stall" });
		}
		let center = Vec3::from(confines.bounds.center());
		let label = LabelNode::rectangle(
			params.style,
			"commercial stall",
			center,
			extent,
			confines.roll,
		);
		Ok(Self {
			parameterized: params,
			label,
		})
	}
}

/// Full commercial stall (Label placeholder).
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStall {
	pub plan: CommercialStallPlan,
}

impl CommercialStall {
	pub fn from_plan(plan: CommercialStallPlan) -> Self {
		Self { plan }
	}

	pub fn label(&self) -> &LabelNode {
		&self.plan.label
	}
}

impl Fit for CommercialStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = CommercialStallParameterized::sample(confines, noise);
		let plan = CommercialStallPlan::from_parameterized(params, confines)?;
		Ok((Self::from_plan(plan), FillableRegions::empty()))
	}
}

impl BuildingComponents for CommercialStall {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::from_free(vec![self.plan.label.clone()])
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	#[test]
	fn stall_fit_emits_label() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(3.0, 3.0, 4.0),
		));
		let (stall, regions) =
			CommercialStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(regions.within.is_empty());
		assert_eq!(stall.label().text, "commercial stall");
		assert!(!stall
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
	}
}
