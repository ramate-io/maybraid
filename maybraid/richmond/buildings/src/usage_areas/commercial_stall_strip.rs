//! Strip of commercial stalls along a gallery band (Label placeholders).

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::{BuildingComponents, LabelNode, Layers};

use crate::fit::{aabb_xz_extent, Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::commercial_stall::{
	CommercialStall, CommercialStallPlan, CommercialStallParameterized,
};

const MIN_STALL_ALONG: f32 = 1.6;
const MAX_STALL_ALONG: f32 = 5.5;
const MIN_STRIP_ALONG: f32 = 1.6;

/// Noise knobs for packing stalls along a strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommercialStallStripParameterized {
	/// Preferred bay width along the strip’s long axis.
	pub bay_width: f32,
}

impl CommercialStallStripParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Self {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let bay = cfg.sample_range_f32_4d(
			MIN_STALL_ALONG,
			MAX_STALL_ALONG,
			c.x,
			c.y,
			c.z,
			21.0,
		);
		Self { bay_width: bay }
	}
}

/// Strip plan: ordered stall cells along the long plan axis.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStallStripPlan {
	pub parameterized: CommercialStallStripParameterized,
	pub stalls: Vec<CommercialStall>,
}

impl CommercialStallStripPlan {
	pub fn from_parameterized(
		params: CommercialStallStripParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let min = Vec3::from(confines.bounds.min);
		let max = Vec3::from(confines.bounds.max);
		let extent = aabb_xz_extent(&confines.bounds);
		let height = (max.y - min.y).max(1e-4);
		if height < 0.4 {
			return Err(FitError::TooSmall { reason: "height" });
		}
		let along_x = extent.x >= extent.y;
		let along = if along_x { extent.x } else { extent.y };
		let depth = if along_x { extent.y } else { extent.x };
		if along < MIN_STRIP_ALONG || depth < 0.4 {
			return Err(FitError::TooSmall { reason: "strip" });
		}

		let bay = params.bay_width.clamp(MIN_STALL_ALONG, along.max(MIN_STALL_ALONG));
		let mut stalls = Vec::new();
		let mut cursor = 0.0_f32;
		let mut i = 0usize;
		while cursor + MIN_STALL_ALONG <= along + 1e-4 {
			let remaining = along - cursor;
			let w = if remaining < bay * 1.35 {
				remaining
			} else {
				bay.min(remaining)
			};
			if w < MIN_STALL_ALONG * 0.85 {
				break;
			}
			let (smin, smax) = if along_x {
				(
					Vec3::new(min.x + cursor, min.y, min.z),
					Vec3::new(min.x + cursor + w, max.y, max.z),
				)
			} else {
				(
					Vec3::new(min.x, min.y, min.z + cursor),
					Vec3::new(max.x, max.y, min.z + cursor + w),
				)
			};
			let cell = Confines::new(
				Aabb3d::from_min_max(smin, smax),
				confines.roll,
				// Openings stay on the strip for later; per-stall subset deferred.
				crate::openings::Openings::new(),
			);
			// Distinct salt per bay via seed offset.
			let mut bay_noise = noise;
			bay_noise.seed = noise.seed.wrapping_add(i as i32 * 17);
			let stall_params = CommercialStallParameterized::sample(&cell, bay_noise);
			let plan = CommercialStallPlan::from_parameterized(stall_params, &cell)?;
			stalls.push(CommercialStall::from_plan(plan));
			cursor += w;
			i += 1;
		}
		if stalls.is_empty() {
			return Err(FitError::TooSmall { reason: "stalls" });
		}
		Ok(Self {
			parameterized: params,
			stalls,
		})
	}
}

/// Full commercial stall strip (Label placeholders).
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStallStrip {
	pub plan: CommercialStallStripPlan,
}

impl CommercialStallStrip {
	pub fn from_plan(plan: CommercialStallStripPlan) -> Self {
		Self { plan }
	}

	pub fn stalls(&self) -> &[CommercialStall] {
		&self.plan.stalls
	}
}

impl Fit for CommercialStallStrip {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = CommercialStallStripParameterized::sample(confines, noise);
		let plan = CommercialStallStripPlan::from_parameterized(params, confines, noise)?;
		Ok((Self::from_plan(plan), FillableRegions::empty()))
	}
}

impl BuildingComponents for CommercialStallStrip {
	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for stall in &self.plan.stalls {
			out.extend(stall.label_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	#[test]
	fn strip_packs_multiple_stalls() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(12.0, 3.5, 5.0),
		));
		let (strip, _) =
			CommercialStallStrip::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(strip.stalls().len() >= 2);
		let labels = strip.label_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(labels.len(), strip.stalls().len());
	}
}
