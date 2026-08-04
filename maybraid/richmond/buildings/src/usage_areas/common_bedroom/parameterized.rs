//! Noise knobs + plan for [`super::CommonBedroom`].

use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::LabelStyle;

use crate::fit::{Confines, FitError};

use super::layout::{CommonBedroomPacked, CommonBedroomRegions};

/// Opening-id scope for closet / ensuite doors authored by this usage area.
pub const SCOPE: &str = "common_bedroom";

/// Noise / fill knobs for [`super::CommonBedroom`].
#[derive(Debug, Clone, PartialEq)]
pub struct CommonBedroomParameterized {
	pub style: LabelStyle,
	/// Multiplier on bed / nightstand / partition base footprints.
	pub spaciousness: f32,
	/// Max floor-area fraction to allocate (leave about `1 - occupancy` empty).
	pub occupancy: f32,
	pub closet_along_t: f32,
	pub ensuite_along_t: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
}

impl CommonBedroomParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();

		let spaciousness = cfg
			.sample_range_f32_4d(0.85, 1.25, c.x, c.y, c.z, 10.0)
			.clamp(0.5, 2.0);
		let occupancy = cfg
			.sample_range_f32_4d(0.35, 0.65, c.x, c.y, c.z, 11.0)
			.clamp(0.05, 1.0);
		let closet_along_t = cfg.sample_range_f32_4d(0.1, 0.9, c.x, c.y, c.z, 12.0);
		let ensuite_along_t = cfg.sample_range_f32_4d(0.1, 0.9, c.x, c.y, c.z, 13.0);
		let door_width = cfg.sample_range_f32_4d(0.7, 1.05, c.x, c.y, c.z, 14.0);
		let door_along_t = cfg.sample_range_f32_4d(0.2, 0.8, c.x, c.y, c.z, 15.0);
		let host_h = (confines.bounds.max.y - confines.bounds.min.y).max(1.0);
		let door_hi = 2.2_f32.min((host_h - 0.25).max(1.8));
		let door_height =
			cfg.sample_range_f32_4d(1.9_f32.min(door_hi), door_hi, c.x, c.y, c.z, 16.0);

		Ok(Self {
			style: LabelStyle::Blue,
			spaciousness,
			occupancy,
			closet_along_t,
			ensuite_along_t,
			door_width,
			door_along_t,
			door_height,
		})
	}

	/// Explicit fill budgets (playground / tests), style fixed.
	pub fn with_fill(spaciousness: f32, occupancy: f32) -> Self {
		Self {
			style: LabelStyle::Blue,
			spaciousness: spaciousness.max(1e-3),
			occupancy: occupancy.clamp(0.05, 1.0),
			closet_along_t: 0.5,
			ensuite_along_t: 0.5,
			door_width: 0.85,
			door_along_t: 0.5,
			door_height: 2.1,
		}
	}
}

/// Sampled knobs + packed geometry for [`super::CommonBedroom`].
#[derive(Debug, Clone, PartialEq)]
pub struct CommonBedroomPlan {
	pub parameterized: CommonBedroomParameterized,
	pub packed: CommonBedroomPacked,
}

impl CommonBedroomPlan {
	pub fn from_parameterized(
		params: CommonBedroomParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let regions = CommonBedroomRegions {
			spaciousness: params.spaciousness,
			occupancy: params.occupancy,
			closet_along_t: params.closet_along_t,
			ensuite_along_t: params.ensuite_along_t,
			door_width: params.door_width,
			door_along_t: params.door_along_t,
			door_height: params.door_height,
		};
		let packed = regions.pack(confines, noise)?;
		Ok(Self {
			parameterized: params,
			packed,
		})
	}
}
