//! I-Apartment full storey: floor plan plus apartments and janitorial fills.
//!
//! The IFloor envelope and halls live on [`IApartmentFloorPlan`]. This type fits
//! that plan, builds [`Apartment`]s from plan-owned groups, and fits
//! [`Janitorial`] slots. Hallways / shafts remain in residual
//! [`FillableRegions::within`]. Apartment interiors stay unfilled.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::usage_areas::{Apartment, Janitorial};

use super::floor_plan::IApartmentFloorPlan;

/// Full I-Apartment storey: shell on [`Self::floor_plan`] plus units + closets.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentFullStorey {
	pub floor_plan: IApartmentFloorPlan,
	pub apartments: Vec<Apartment>,
	pub janitorial: Vec<Janitorial>,
}

impl IApartmentFullStorey {
	/// Wrap an already-fitted floor plan and fill apartment groups + janitorial.
	pub fn from_floor_plan(
		floor_plan: IApartmentFloorPlan,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let regions = floor_plan.fillable_regions();
		Self::fill_from_plan(floor_plan, regions, noise)
	}

	fn fill_from_plan(
		floor_plan: IApartmentFloorPlan,
		regions: FillableRegions,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let mut apartments = Vec::new();
		let mut residual_within = Vec::new();

		// Prefer plan-owned groups (multi-cell source of truth).
		for group in floor_plan.apartment_groups() {
			let pieces: Vec<(u32, Confines)> = group
				.cell_ids
				.iter()
				.zip(group.pieces.iter())
				.map(|(id, c)| (*id, c.clone()))
				.collect();
			match Apartment::from_pieces(group.group_id, pieces) {
				Ok((apt, nested)) => {
					apartments.push(apt);
					// Keep nested piece residuals for later interior fill.
					residual_within.extend(nested.within);
				}
				Err(FitError::TooSmall { .. }) => {
					for piece in &group.pieces {
						residual_within.push(FillRegion::new(
							SpaceKind::Custom(format!("apartment:{}", group.group_id)),
							piece.clone(),
						));
					}
				}
				Err(err) => return Err(err),
			}
		}

		let mut janitorial = Vec::new();
		for (i, slot) in floor_plan.janitorial_slots().iter().enumerate() {
			let mut slot_noise = noise;
			slot_noise.seed = noise.seed.wrapping_add(i as i32 * 17);
			match Janitorial::fit_to_confines(slot, slot_noise) {
				Ok((j, _)) => janitorial.push(j),
				Err(FitError::TooSmall { .. }) => {
					residual_within.push(FillRegion::new(
						SpaceKind::Custom("janitorial".into()),
						slot.clone(),
					));
				}
				Err(err) => return Err(err),
			}
		}

		// Pass through non-apartment / non-janitorial residuals from the plan.
		for region in regions.within {
			match &region.kind {
				SpaceKind::Custom(label)
					if label.starts_with("apartment:") || label == "janitorial" =>
				{
					// Already handled via plan-owned lists.
				}
				_ => residual_within.push(region),
			}
		}

		Ok((
			Self {
				floor_plan,
				apartments,
				janitorial,
			},
			FillableRegions {
				within: residual_within,
				atop: regions.atop,
			},
		))
	}
}

impl Fit for IApartmentFullStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (floor_plan, regions) = IApartmentFloorPlan::fit_to_confines(confines, noise)?;
		Self::fill_from_plan(floor_plan, regions, noise)
	}
}

impl BuildingComponents for IApartmentFullStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		// Floor plan already owns envelope + apartment/janitorial shells.
		self.floor_plan.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.floor_plan.joint_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		self.floor_plan.label_nodes_for_level(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use crate::storeys::i_apartment::{IApartmentFloorPlan, IApartmentParameterized};

	#[test]
	fn full_storey_builds_apartments() {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		);
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = IApartmentParameterized::sample(&empty, noise).unwrap();
		let openings = IApartmentFloorPlan::shaft_requests_for_all_slots(&params, &empty);
		let confines = Confines::new(bounds, 0.0, openings);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		let (storey, regions) = IApartmentFullStorey::from_floor_plan(plan, noise).unwrap();
		assert!(!storey.apartments.is_empty());
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::Hallway));
		assert!(!storey
			.panel_nodes_for_level(LodSceneLevel::High)
			.is_empty());
	}
}
