//! Parameterized knobs + plan for [`super::EatingArea`].

use bevy_math::bounding::Aabb2d;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::LabelStyle;

use crate::fit::{Confines, FitError};
use crate::usage_areas::livable_quarters::dining_room::DiningRoomPlan;
use crate::usage_areas::livable_quarters::kitchen::{KitchenCounterLayout, KitchenPlan};

pub const SCOPE: &str = "eating_area";

/// Combined kitchen + optional dining layout.
#[derive(Debug, Clone, PartialEq)]
pub struct EatingAreaParameterized {
	pub style: LabelStyle,
	pub spaciousness: f32,
	pub occupancy: f32,
	/// Fraction of footprint claimed by the kitchen when splitting (0–1).
	pub kitchen_frac: f32,
	pub kitchen_layout: Option<KitchenCounterLayout>,
}

impl EatingAreaParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		Ok(Self {
			style: LabelStyle::Yellow,
			spaciousness: cfg
				.sample_range_f32_4d(0.95, 1.35, c.x, c.y, c.z, 40.0)
				.clamp(0.75, 1.75),
			occupancy: cfg
				.sample_range_f32_4d(0.28, 0.52, c.x, c.y, c.z, 41.0)
				.clamp(0.1, 0.75),
			kitchen_frac: cfg
				.sample_range_f32_4d(0.38, 0.58, c.x, c.y, c.z, 42.0)
				.clamp(0.3, 0.7),
			kitchen_layout: None,
		})
	}

	pub fn with_fill(spaciousness: f32, occupancy: f32) -> Self {
		Self {
			style: LabelStyle::Yellow,
			spaciousness: spaciousness.max(1e-3),
			occupancy: occupancy.clamp(0.05, 1.0),
			kitchen_frac: 0.45,
			kitchen_layout: None,
		}
	}
}

/// Chosen layout after a successful pack.
#[derive(Debug, Clone, PartialEq)]
pub enum EatingAreaPacked {
	KitchenDining {
		kitchen: KitchenPlan,
		dining: DiningRoomPlan,
		kitchen_xz: Aabb2d,
		dining_xz: Aabb2d,
	},
	KitchenOnly {
		kitchen: KitchenPlan,
	},
}

#[derive(Debug, Clone, PartialEq)]
pub struct EatingAreaPlan {
	pub parameterized: EatingAreaParameterized,
	pub packed: EatingAreaPacked,
}
