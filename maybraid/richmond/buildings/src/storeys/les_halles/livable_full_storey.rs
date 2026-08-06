//! Les Halles livable full storey: ring floor plan + gallery [`LivableApartments`].
//!
//! Reuses [`LesHallesFloorPlan`] (same shell / strip residuals as the commercial
//! Full\*). Each [`SpaceKind::ExternalSpace`] gallery strip is an independent
//! rectangle along the ring — no progressive [`OpeningLabel::Boundary`] handoff
//! (strips do not share living walls; shafts own the corners).
//!
//! Before fill, abutting plan shafts are injected onto the strip confines so
//! [`HallsToShafts`] has terminals alongside balcony-facing Passage doors.

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers};

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::openings::{Opening, OpeningId, OpeningLabel};
use crate::usage_areas::plan_geom::{host_xz, noise_for_cell};
use crate::usage_areas::{
	LivableApartments, LivableApartmentsOptions, MAX_HALL_WIDTH, MIN_HALL_WIDTH,
};

use super::floor_plan::LesHallesFloorPlan;
use super::parameterized::LesHallesParameterized;
use super::SCOPE;

const EPS: f32 = 1e-3;
/// How far a shaft stub reaches into a strip when they only share an edge.
const SHAFT_STUB_DEPTH: f32 = 0.5;
/// Plan-space touch tolerance when matching shafts to gallery strips.
const ABUT_EPS: f32 = 0.05;
const SALT_HALL_WIDTH: f32 = 120.0;

/// Full Les Halles storey with residential gallery fills.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesLivableFullStorey {
	pub floor_plan: LesHallesFloorPlan,
	/// One [`LivableApartments`] pack per filled gallery strip.
	pub blocks: Vec<LivableApartments>,
	/// Corridor clear width passed into each strip pack.
	pub hall_width: f32,
}

impl LesHallesLivableFullStorey {
	/// Wrap an already-fitted floor plan and fill external gallery strips.
	pub fn from_floor_plan(
		floor_plan: LesHallesFloorPlan,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let regions = floor_plan.fillable_regions();
		Self::fill_from_regions(floor_plan, regions, noise)
	}

	fn fill_from_regions(
		floor_plan: LesHallesFloorPlan,
		regions: FillableRegions,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let hall_width = sample_hall_width(&floor_plan, noise);
		let opts = LivableApartmentsOptions {
			hall_width: Some(hall_width),
			targets: None,
		};

		let mut blocks = Vec::new();
		let mut residual_within = Vec::new();
		let mut strip_i = 0i32;
		for region in regions.within {
			if region.kind != SpaceKind::ExternalSpace {
				residual_within.push(region);
				continue;
			}
			let mut strip_confines = region.confines;
			inject_abutting_shafts(
				&mut strip_confines,
				&floor_plan.shaft_bounds,
				&floor_plan.shaft_slots,
			);
			let block_noise = noise_for_cell(noise, strip_i);
			strip_i += 1;
			match LivableApartments::from_confines_with(&strip_confines, block_noise, opts.clone())
			{
				Ok((block, nested)) => {
					blocks.push(block);
					residual_within.extend(nested.within.into_iter().map(as_closet_if_internal));
				}
				Err(FitError::TooSmall { .. }) => {
					// Leave unfilled if the strip is too narrow after shaft clears.
					residual_within.push(FillRegion::new(
						SpaceKind::ExternalSpace,
						strip_confines,
					));
				}
				Err(err) => return Err(err),
			}
		}

		Ok((
			Self {
				floor_plan,
				blocks,
				hall_width,
			},
			FillableRegions {
				within: residual_within,
				atop: regions.atop,
			},
		))
	}
}

fn sample_hall_width(floor_plan: &LesHallesFloorPlan, noise: NoiseParams) -> f32 {
	let cfg = NoiseConfig::new(noise);
	let c = floor_plan.center_xz;
	cfg.sample_range_f32_4d(
		MIN_HALL_WIDTH,
		MAX_HALL_WIDTH,
		c.x,
		c.y,
		c.z,
		SALT_HALL_WIDTH,
	)
	.clamp(MIN_HALL_WIDTH * 0.5, MAX_HALL_WIDTH * 1.5)
}

fn as_closet_if_internal(region: FillRegion) -> FillRegion {
	match region.kind {
		SpaceKind::InternalSpace => FillRegion::new(SpaceKind::ClosetSpace, region.confines),
		_ => region,
	}
}

/// Inject scoped shaft openings for plan shafts that abut the strip.
fn inject_abutting_shafts(
	strip: &mut Confines,
	shaft_bounds: &[Aabb3d],
	shaft_slots: &[usize],
) {
	let strip_xz = host_xz(&strip.bounds);
	let y0 = Vec3::from(strip.bounds.min).y;
	let y1 = Vec3::from(strip.bounds.max).y;
	for (i, shaft) in shaft_bounds.iter().enumerate() {
		let Some(stub_xz) = shaft_stub_into_strip(host_xz(shaft), strip_xz) else {
			continue;
		};
		let slot = shaft_slots.get(i).copied().unwrap_or(i);
		let bounds = Aabb3d::from_min_max(
			Vec3::new(stub_xz.min.x, y0, stub_xz.min.y),
			Vec3::new(stub_xz.max.x, y1, stub_xz.max.y),
		);
		strip.openings.insert(
			OpeningId::scoped(SCOPE, "strip_shaft", slot.to_string()),
			Opening::new(bounds, OpeningLabel::Shaft),
		);
	}
}

/// Plan stub of a shaft that reaches into `strip` (overlap or edge abutment).
fn shaft_stub_into_strip(shaft: Aabb2d, strip: Aabb2d) -> Option<Aabb2d> {
	if !aabb2_touches(shaft, strip, ABUT_EPS) {
		return None;
	}
	let overlap = Aabb2d {
		min: Vec2::new(shaft.min.x.max(strip.min.x), shaft.min.y.max(strip.min.y)),
		max: Vec2::new(shaft.max.x.min(strip.max.x), shaft.max.y.min(strip.max.y)),
	};
	let ow = overlap.max.x - overlap.min.x;
	let oh = overlap.max.y - overlap.min.y;
	if ow > EPS && oh > EPS {
		return Some(overlap);
	}

	// Edge abutment: push a thin stub from the shared edge into the strip.
	let depth = SHAFT_STUB_DEPTH;
	let stub = if ow > EPS && oh <= EPS {
		// Shared edge parallel to X.
		let mid_z = overlap.min.y.clamp(strip.min.y, strip.max.y);
		let into_pos = strip.center().y >= mid_z;
		let (z0, z1) = if into_pos {
			(mid_z, (mid_z + depth).min(strip.max.y))
		} else {
			((mid_z - depth).max(strip.min.y), mid_z)
		};
		Aabb2d {
			min: Vec2::new(overlap.min.x, z0),
			max: Vec2::new(overlap.max.x, z1),
		}
	} else if oh > EPS && ow <= EPS {
		// Shared edge parallel to Z.
		let mid_x = overlap.min.x.clamp(strip.min.x, strip.max.x);
		let into_pos = strip.center().x >= mid_x;
		let (x0, x1) = if into_pos {
			(mid_x, (mid_x + depth).min(strip.max.x))
		} else {
			((mid_x - depth).max(strip.min.x), mid_x)
		};
		Aabb2d {
			min: Vec2::new(x0, overlap.min.y),
			max: Vec2::new(x1, overlap.max.y),
		}
	} else {
		// Corner touch: take a small square at the nearest strip corner toward shaft.
		let cx = shaft.center().x.clamp(strip.min.x, strip.max.x);
		let cz = shaft.center().y.clamp(strip.min.y, strip.max.y);
		let half = depth * 0.5;
		Aabb2d {
			min: Vec2::new((cx - half).max(strip.min.x), (cz - half).max(strip.min.y)),
			max: Vec2::new((cx + half).min(strip.max.x), (cz + half).min(strip.max.y)),
		}
	};
	if stub.max.x - stub.min.x > EPS && stub.max.y - stub.min.y > EPS {
		Some(stub)
	} else {
		None
	}
}

fn aabb2_touches(a: Aabb2d, b: Aabb2d, eps: f32) -> bool {
	let x_overlap = a.min.x < b.max.x + eps && a.max.x > b.min.x - eps;
	let y_overlap = a.min.y < b.max.y + eps && a.max.y > b.min.y - eps;
	x_overlap && y_overlap
}

impl Fit for LesHallesLivableFullStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = LesHallesParameterized::sample_livable(confines, noise)?;
		let (floor_plan, regions) = LesHallesFloorPlan::from_parameterized(params, confines)?;
		Self::fill_from_regions(floor_plan, regions, noise)
	}
}

impl BuildingComponents for LesHallesLivableFullStorey {
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

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		for block in &self.blocks {
			for apt in &block.apartments {
				out.extend(apt.furniture_nodes_for_level(level));
			}
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

	fn structural_lod_probe(&self) -> Option<BuildingStructuralLodProbe> {
		let mut probe: Option<BuildingStructuralLodProbe> = None;
		for block in &self.blocks {
			let Some(block_probe) = block.structural_lod_probe() else {
				continue;
			};
			probe = Some(match probe {
				Some(acc) => acc.merge(block_probe),
				None => block_probe,
			});
		}
		probe
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use lod::gen::LodSceneLevel;
	use procedural_common::NoiseParams;
	use richmond_building_components::BuildingComponents;

	fn large_bounds() -> Aabb3d {
		Aabb3d::from_min_max(
			Vec3::new(-36.0, 0.0, -27.0),
			Vec3::new(36.0, 4.0, 27.0),
		)
	}

	fn storey_with_shafts(seed: i32) -> (LesHallesLivableFullStorey, FillableRegions) {
		let bounds = large_bounds();
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams {
			seed,
			..NoiseParams::default()
		};
		let params = LesHallesParameterized::sample_livable(&empty, noise).unwrap();
		let openings = LesHallesFloorPlan::shaft_requests_for_all_slots(&params, &empty);
		let confines = Confines::new(bounds, 0.0, openings);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(params, &confines).unwrap();
		LesHallesLivableFullStorey::from_floor_plan(plan, noise).unwrap()
	}

	#[test]
	fn livable_full_storey_fills_external_strips() {
		let (storey, regions) = storey_with_shafts(1337);
		assert!(!storey.blocks.is_empty());
		assert!(storey.hall_width + 1e-3 >= MIN_HALL_WIDTH * 0.5);
		assert!(regions
			.within
			.iter()
			.all(|r| r.kind != SpaceKind::ExternalSpace));
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::Walkway));
		assert_eq!(regions.atop.len(), 1);
		assert!(storey.floor_plan.gallery.wall_count() >= 4);
		assert!(!storey
			.panel_nodes_for_level(LodSceneLevel::High)
			.is_empty());
		assert!(!storey
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
	}

	#[test]
	fn livable_blocks_have_halls_or_apartments() {
		let (storey, _) = storey_with_shafts(7);
		assert!(
			storey.blocks.iter().any(|b| !b.apartments.is_empty())
				|| storey.blocks.iter().any(|b| !b.halls.hall_bands.is_empty()),
			"expected apartments or carved halls in at least one strip"
		);
	}

	#[test]
	fn inject_abutting_shafts_adds_stub_on_shared_edge() {
		let mut strip = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(20.0, 3.0, 8.0),
		));
		let shaft = Aabb3d::from_min_max(
			Vec3::new(20.0, 0.0, 0.0),
			Vec3::new(26.0, 3.0, 6.0),
		);
		inject_abutting_shafts(&mut strip, &[shaft], &[2]);
		let id = OpeningId::scoped(SCOPE, "strip_shaft", "2");
		let opening = strip.openings.get(&id).expect("injected shaft");
		assert!(matches!(opening.label, OpeningLabel::Shaft));
		let stub = host_xz(&opening.bounds);
		assert!(stub.min.x + EPS < 20.0, "stub should enter the strip");
		assert!(stub.max.x <= 20.0 + EPS);
	}

	#[test]
	fn too_small_strip_left_as_external_residual() {
		// Tiny host cannot pack LivableApartments; fill path should not hard-fail.
		let floor_plan = {
			let bounds = large_bounds();
			let empty = Confines::from_bounds(bounds);
			let noise = NoiseParams::default();
			let params = LesHallesParameterized::sample_livable(&empty, noise).unwrap();
			let openings = LesHallesFloorPlan::shaft_requests_for_all_slots(&params, &empty);
			let confines = Confines::new(bounds, 0.0, openings);
			LesHallesFloorPlan::from_parameterized(params, &confines)
				.unwrap()
				.0
		};
		let tiny = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 3.0, 2.0),
		));
		let regions = FillableRegions {
			within: vec![FillRegion::new(SpaceKind::ExternalSpace, tiny)],
			atop: Vec::new(),
		};
		let (storey, residual) =
			LesHallesLivableFullStorey::fill_from_regions(floor_plan, regions, NoiseParams::default())
				.unwrap();
		assert!(storey.blocks.is_empty());
		assert!(residual
			.within
			.iter()
			.any(|r| r.kind == SpaceKind::ExternalSpace));
	}
}
