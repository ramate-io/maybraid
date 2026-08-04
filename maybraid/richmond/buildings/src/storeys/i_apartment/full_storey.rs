//! I-Apartment full storey: floor plan → [`HallsToShafts`] → livable rooms.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError, SpaceKind};
use crate::usage_areas::{HallsToShafts, LivableApartment};

use super::floor_plan::IApartmentFloorPlan;

/// Full I-Apartment storey: I-frame shell, halls-to-shafts, and livable rooms.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentFullStorey {
	pub floor_plan: IApartmentFloorPlan,
	pub halls: Vec<HallsToShafts>,
	pub apartments: Vec<LivableApartment>,
}

impl IApartmentFullStorey {
	/// Wrap an already-fitted floor plan and allocate halls + livable apartments.
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
		let mut halls = Vec::new();
		let mut apartments = Vec::new();
		let mut residual_within = Vec::new();
		let mut next_apt_id = 0u32;

		for region in regions.within {
			match HallsToShafts::fit_to_confines(&region.confines, noise) {
				Ok((hts, nested)) => {
					halls.push(hts);
					for nested_region in nested.within {
						match nested_region.kind {
							SpaceKind::InternalSpace => {
								match LivableApartment::from_confines(
									next_apt_id,
									&nested_region.confines,
								) {
									Ok((apt, apt_nested)) => {
										next_apt_id = next_apt_id.saturating_add(1);
										apartments.push(apt);
										residual_within.extend(apt_nested.within);
									}
									Err(FitError::TooSmall { .. }) => {
										residual_within.push(nested_region);
									}
									Err(err) => return Err(err),
								}
							}
							SpaceKind::Hallway => {
								residual_within.push(nested_region);
							}
							_ => residual_within.push(nested_region),
						}
					}
				}
				Err(FitError::TooSmall { .. }) => {
					// Fall back: treat the whole primary rect as one livable.
					match LivableApartment::from_confines(next_apt_id, &region.confines) {
						Ok((apt, nested)) => {
							next_apt_id = next_apt_id.saturating_add(1);
							apartments.push(apt);
							residual_within.extend(nested.within);
						}
						Err(FitError::TooSmall { .. }) => {
							residual_within.push(region);
						}
						Err(err) => return Err(err),
					}
				}
				Err(err) => return Err(err),
			}
		}

		Ok((
			Self {
				floor_plan,
				halls,
				apartments,
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
		// Outer I-frame from the floor plan; apartments add their region shells.
		let mut out = self.floor_plan.panel_nodes_for_level(level);
		for apt in &self.apartments {
			out.extend(apt.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.floor_plan.joint_nodes_for_level(level);
		for apt in &self.apartments {
			out.extend(apt.joint_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = self.floor_plan.label_nodes_for_level(level);
		for apt in &self.apartments {
			out.extend(apt.label_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use crate::storeys::i_apartment::{IApartmentFloorPlan, IApartmentParameterized};

	#[test]
	fn full_storey_allocates_livable_per_rect() {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		);
		let confines = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = IApartmentParameterized::sample(&confines, noise).unwrap();
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		let n = plan.primary_rects.len();
		let (storey, _) = IApartmentFullStorey::from_floor_plan(plan, noise).unwrap();
		// No shafts/passages → HallsToShafts is a no-op carve; one room per rect.
		assert_eq!(storey.halls.len(), n);
		assert_eq!(storey.apartments.len(), n);
		assert!(!storey
			.panel_nodes_for_level(LodSceneLevel::High)
			.is_empty());
	}

	#[test]
	fn full_storey_carves_halls_when_shafts_present() {
		use crate::openings::{Opening, OpeningId, OpeningLabel};

		let bounds = Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		);
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = IApartmentParameterized::sample(&empty, noise).unwrap();
		let inbound = IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty);
		let confines = Confines::new(bounds, 0.0, inbound);
		let (mut plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		// One shaft per rect alone does not form a hall; add a passage so
		// HallsToShafts has two terminals on the first primary rect.
		if let Some(rect) = plan.primary_rects.first() {
			let cx = 0.5 * (rect.min_x + rect.max_x);
			let cz = rect.max_z - 0.2;
			plan.openings.insert(
				OpeningId::new("test_passage"),
				Opening::new(
					Aabb3d::from_min_max(
						Vec3::new(cx - 0.6, 0.0, cz - 0.15),
						Vec3::new(cx + 0.6, 3.5, cz + 0.15),
					),
					OpeningLabel::Passage,
				),
			);
		}
		let (storey, residual) = IApartmentFullStorey::from_floor_plan(plan, noise).unwrap();
		assert!(!storey.halls.is_empty());
		assert!(
			storey.halls.iter().any(|h| !h.hall_bands.is_empty())
				|| residual
					.within
					.iter()
					.any(|r| r.kind == SpaceKind::Hallway),
			"expected at least one carved hallway when shaft+passage present"
		);
		assert!(!storey.apartments.is_empty());
	}
}
