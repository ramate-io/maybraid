//! I-Apartment full storey: floor plan → [`LivableApartments`] per primary rect.
//!
//! Primary rects are filled **progressively**: each block may wall shared
//! interfaces with later siblings, then injects [`OpeningLabel::Boundary`] onto
//! those siblings so they do not double-wall.

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::usage_areas::boundary_openings::inject_shared_boundary_from;
use crate::usage_areas::plan_geom::{host_xz, noise_for_cell};
use crate::usage_areas::{LivableApartments, LivableApartmentsOptions};

use super::floor_plan::IApartmentFloorPlan;
use super::SCOPE;

/// Full I-Apartment storey: I-frame shell + livable apartment blocks per primary rect.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentFullStorey {
	pub floor_plan: IApartmentFloorPlan,
	/// One [`LivableApartments`] pack per primary rectangular residual.
	pub blocks: Vec<LivableApartments>,
}

impl IApartmentFullStorey {
	/// Wrap an already-fitted floor plan and allocate livable apartment blocks.
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
		let mut blocks = Vec::new();
		let mut residual_within = Vec::new();
		let opts = LivableApartmentsOptions {
			hall_width: Some(floor_plan.hall_width),
			targets: None,
		};

		// Progressive fill: each primary rect may claim shared edges, then marks
		// later siblings with Boundary so they skip those faces.
		let mut pending: Vec<FillRegion> = regions.within;
		for i in 0..pending.len() {
			// Match Les Halles: diversify seed per primary rect so stem/flange
			// packs and room programs do not clone each other.
			let block_noise = noise_for_cell(noise, i as i32);
			match LivableApartments::from_confines_with(
				&pending[i].confines,
				block_noise,
				opts.clone(),
			) {
				Ok((block, nested)) => {
					let owner = host_xz(&pending[i].confines.bounds);
					for j in (i + 1)..pending.len() {
						inject_shared_boundary_from(
							owner,
							&mut pending[j].confines,
							SCOPE,
							format!("prog_{i}_{j}"),
						);
					}
					blocks.push(block);
					residual_within.extend(nested.within.into_iter().map(as_closet_if_internal));
				}
				Err(FitError::TooSmall { .. }) => {
					residual_within.push(FillRegion::new(
						SpaceKind::ClosetSpace,
						pending[i].confines.clone(),
					));
				}
				Err(err) => return Err(err),
			}
		}

		Ok((
			Self {
				floor_plan,
				blocks,
			},
			FillableRegions {
				within: residual_within,
				atop: regions.atop,
			},
		))
	}
}

fn as_closet_if_internal(region: FillRegion) -> FillRegion {
	match region.kind {
		SpaceKind::InternalSpace => FillRegion::new(SpaceKind::ClosetSpace, region.confines),
		_ => region,
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
		let mut out = self.floor_plan.panel_nodes_for_level(level);
		for block in &self.blocks {
			out.extend(block.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.floor_plan.joint_nodes_for_level(level);
		for block in &self.blocks {
			out.extend(block.joint_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = self.floor_plan.label_nodes_for_level(level);
		for block in &self.blocks {
			out.extend(block.label_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use crate::openings::OpeningLabel;
	use crate::storeys::i_apartment::{IApartmentFloorPlan, IApartmentParameterized};
	use crate::usage_areas::plan_cells::{hall_frontage_length, PlanCell, MIN_GROUP_CONNECTIVITY};
	use crate::usage_areas::plan_geom::host_xz;

	fn storey_seed(seed: i32) -> IApartmentFullStorey {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		);
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams {
			seed,
			..NoiseParams::default()
		};
		let params = IApartmentParameterized::sample(&empty, noise).unwrap();
		let inbound = IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty);
		let confines = Confines::new(bounds, 0.0, inbound);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		IApartmentFullStorey::from_floor_plan(plan, noise).unwrap().0
	}

	#[test]
	fn full_storey_allocates_block_per_rect() {
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
		assert_eq!(storey.blocks.len(), n);
		assert!(!storey
			.panel_nodes_for_level(LodSceneLevel::High)
			.is_empty());
	}

	#[test]
	fn full_storey_blocks_contain_apartments_when_connected() {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		);
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = IApartmentParameterized::sample(&empty, noise).unwrap();
		let inbound = IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty);
		let confines = Confines::new(bounds, 0.0, inbound);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		let (storey, _) = IApartmentFullStorey::from_floor_plan(plan, noise).unwrap();
		assert!(!storey.blocks.is_empty());
		assert!(
			storey.blocks.iter().any(|b| !b.apartments.is_empty())
				|| storey.blocks.iter().any(|b| !b.halls.hall_bands.is_empty()),
			"expected apartments or carved halls in at least one block"
		);
	}

	#[test]
	fn seed_1337_apartments_have_real_hall_frontage() {
		let storey = storey_seed(1337);
		for block in &storey.blocks {
			for apt in &block.apartments {
				let mut best = 0.0_f32;
				for part in apt.cells.iter() {
					let cell = PlanCell::new(0, host_xz(&part.confines.bounds));
					best = best.max(hall_frontage_length(
						&cell,
						&block.halls.hall_bands,
						1e-3,
					));
				}
				assert!(
					best + 1e-3 >= MIN_GROUP_CONNECTIVITY,
					"apartment hall frontage {best:.3} < {MIN_GROUP_CONNECTIVITY}"
				);
			}
		}
	}

	#[test]
	fn seed_1337_later_block_inherits_progressive_boundary() {
		let storey = storey_seed(1337);
		if storey.blocks.len() < 2 {
			return;
		}
		// Flange (later) should carry Boundary openings from progressive handoff
		// and/or exterior marking.
		let flange = &storey.blocks[1];
		let has_boundary = flange
			.confines
			.openings
			.iter()
			.any(|(_, o)| matches!(o.label, OpeningLabel::Boundary));
		assert!(
			has_boundary,
			"expected Boundary openings on progressive/exterior faces"
		);
		// Stem should author some enclosure walls (shared interface + hall).
		assert!(
			!storey.blocks[0].walls.is_empty(),
			"stem block should wall its claimed interfaces"
		);
	}
}
