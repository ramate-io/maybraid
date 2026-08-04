//! Parameterized knobs + fit for [`super::BitesSitdownStall`].
//!
//! Composes [`BitesStallParameterized`] for counters/style, then reserves seating
//! before the kitchen remainder.

use bevy_math::bounding::Aabb3d;
use procedural_common::{aabb2_area, aabb3_to_plan, NoiseConfig, NoiseParams, PlanAxes};
use richmond_building_components::LabelStyle;

use crate::fit::{Confines, FitError};

use super::super::bites_stall::BitesStallParameterized;
use super::super::stall_layout::{
	pack_bites_sitdown_regions, BITES_REGION_MIN_PLAN, BITES_SEATING_FACE_CONTACT,
};

/// Noise / style knobs for [`super::BitesSitdownStall`].
#[derive(Debug, Clone, PartialEq)]
pub struct BitesSitdownParameterized {
	/// Shared counter / label-style knobs from the plain bites stall.
	pub base: BitesStallParameterized,
	/// Target seating plan area (m²). Sampled from noise; fit grows toward it.
	pub seating_area_target: f32,
	/// Inward seed depth from the opening face (≥1m).
	pub seating_seed_depth: f32,
	/// 0..1 placement of the ≥1m face contact along the free segment.
	pub seating_along_t: f32,
}

impl BitesSitdownParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let base = BitesStallParameterized::sample(confines, noise)?;
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let usable = aabb2_area(aabb3_to_plan(&confines.bounds, PlanAxes::XZ)).max(1.0);
		// Bias seating toward a substantial share of the free plan.
		let lo = (usable * 0.38).max(4.0).min(usable.max(4.0));
		let hi = (usable * 0.62).max(lo + 0.5);
		let seating_area_target = cfg.sample_range_f32_4d(lo, hi, c.x, c.y, c.z, 44.0);
		let seating_seed_depth = cfg.sample_range_f32_4d(1.2, 2.4, c.x, c.y, c.z, 45.0);
		let seating_along_t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 46.0);
		Ok(Self {
			base,
			seating_area_target,
			seating_seed_depth,
			seating_along_t,
		})
	}

	pub fn style(&self) -> LabelStyle {
		self.base.style
	}
}

/// Geometry resolved from [`BitesSitdownParameterized`].
#[derive(Debug, Clone, PartialEq)]
pub struct BitesSitdownPlan {
	pub parameterized: BitesSitdownParameterized,
	pub counter_aabbs: Vec<Aabb3d>,
	pub seating_aabb: Aabb3d,
	pub kitchen_aabb: Aabb3d,
}

impl BitesSitdownPlan {
	pub fn from_parameterized(
		params: BitesSitdownParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let packed = params.base.pack_counters(confines)?;
		let (seating_aabb, kitchen_aabb) = pack_bites_sitdown_regions(
			&confines.bounds,
			&packed.counters,
			&packed.faces,
			params.seating_area_target,
			BITES_SEATING_FACE_CONTACT,
			params.seating_seed_depth,
			params.seating_along_t,
			BITES_REGION_MIN_PLAN,
		)
		.ok_or(FitError::TooSmall {
			reason: "bites seating/kitchen",
		})?;
		Ok(Self {
			parameterized: params,
			counter_aabbs: packed.counters,
			seating_aabb,
			kitchen_aabb,
		})
	}
}
