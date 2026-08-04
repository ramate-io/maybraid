//! I-Apartment floor plan: IFloor envelope + allocated regions (no internal walls).
//!
//! Presentation is the outer [`IFloor`] only — facade apertures cut the walls,
//! hallways / shafts cut the floor slab. Apartment / janitorial / shaft cells are
//! tracked as residuals + labels so we can layer structure later.

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::{aabb3_to_plan, plan_to_aabb3, PlanAxes};
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::{LabelNode, LabelStyle};
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{
	aabb_xz_extent, aabb_xz_overlap_area, Confines, FillRegion, FillableRegions, Fit, FitError,
	SpaceKind, StackRegion,
};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::fitted_rectangle::FittedRectangle;
use crate::paneling::panel_complex::{PanelPoint, DEFAULT_PANEL_THICKNESS};
use crate::shells::ortho::{PlanRect, WallEdge};
use crate::shells::{IFloor, IFloorParams, IFloorPlanRect, IFloorSlab};
use crate::usage_areas::plan_cells::{
	cell_has_hall_frontage, group_cells_to_apartments, split_toward_min_room, subtract_aabb2,
	PlanCell,
};
use crate::usage_areas::Janitorial;

use super::parameterized::{IApartmentParameterized, MIN_STOREY_HEIGHT};
use super::SCOPE;

const EPS: f32 = 1e-3;
/// Fixed shaft pocket side on the I skeleton (meters).
const SHAFT_SIDE: f32 = 2.4;

/// One apartment group authored by the floor plan (source of truth for Full\*).
#[derive(Debug, Clone, PartialEq)]
pub struct ApartmentGroup {
	pub group_id: u32,
	pub cell_ids: Vec<u32>,
	pub pieces: Vec<Confines>,
}

/// I-Apartment floor plan: envelope + circulation + residual groups.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentFloorPlan {
	pub parameterized: IApartmentParameterized,
	pub center_xz: Vec3,
	pub storey_height: f32,
	pub roll: f32,
	pub openings: Openings,
	pub shell: IFloor,
	pub hall_bounds: Vec<Aabb3d>,
	pub shaft_bounds: Vec<Aabb3d>,
	/// Active skeleton shaft slot indices.
	pub shaft_slots: Vec<usize>,
	pub shaft_inbound: Vec<Vec<OpeningId>>,
	pub apartment_groups: Vec<ApartmentGroup>,
	pub janitorial_slots: Vec<Confines>,
	/// Floor fill for allocated rooms only (halls / shafts left open).
	pub region_floors: Vec<FittedRectangle>,
}

impl IApartmentFloorPlan {
	pub fn from_parameterized(
		params: IApartmentParameterized,
		confines: &Confines,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_parameterized_with_ceiling(params, confines, IFloorSlab::None)
	}

	pub fn from_parameterized_with_ceiling(
		params: IApartmentParameterized,
		confines: &Confines,
		ceiling: IFloorSlab,
	) -> Result<(Self, FillableRegions), FitError> {
		let height = (confines.bounds.max.y - confines.bounds.min.y).max(0.0);
		if height < MIN_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}
		let y0 = confines.bounds.min.y;
		let center = confines.center();
		let center_xz = Vec3::new(center.x, y0, center.z);

		let ifloor_params = derive_ifloor_params(&params, confines, center_xz, height, ceiling)?;
		let primary = ifloor_params.plan_rects();
		if primary.is_empty() {
			return Err(FitError::TooSmall { reason: "i_rects" });
		}

		let mut openings = confines.openings.clone();

		// --- shafts: fixed reentrant-corner slots on the I skeleton ---
		let candidates = skeleton_shaft_candidates(&ifloor_params, height, y0);
		let inbound_by_slot = map_inbound_shafts(&mut openings, &candidates);
		let mut shaft_bounds = Vec::new();
		let mut shaft_slots = Vec::new();
		let mut shaft_inbound = Vec::new();
		let mut shaft_excludes: Vec<Aabb2d> = Vec::new();
		for (slot, ids) in inbound_by_slot.into_iter().enumerate() {
			if ids.is_empty() {
				continue;
			}
			let bounds = candidates[slot];
			shaft_slots.push(slot);
			shaft_bounds.push(bounds);
			shaft_inbound.push(ids);
			shaft_excludes.push(aabb3_to_plan(&bounds, PlanAxes::XZ));
		}

		// --- halls: spine per primary rect + stubs to active shafts / portals ---
		let mut hall_plans: Vec<Aabb2d> = Vec::new();
		let mut rect_spines: Vec<Option<Aabb2d>> = Vec::with_capacity(primary.len());
		for rect in &primary {
			let host = rect.to_aabb2();
			let spine = spine_band(host, params.hall_width, params.spine_offset);
			let Some(spine) = spine else {
				rect_spines.push(None);
				continue;
			};
			hall_plans.push(spine);
			rect_spines.push(Some(spine));
		}

		for (si, shaft) in shaft_bounds.iter().enumerate() {
			let shaft2 = aabb3_to_plan(shaft, PlanAxes::XZ);
			// Stub from nearest spine.
			let mut best: Option<(usize, f32)> = None;
			for (ri, spine) in rect_spines.iter().enumerate() {
				let Some(spine) = *spine else { continue };
				let sc = 0.5 * (spine.min + spine.max);
				let tc = 0.5 * (shaft2.min + shaft2.max);
				let dist = sc.distance(tc);
				if best.is_none_or(|(_, d)| dist < d) {
					best = Some((ri, dist));
				}
			}
			if let Some((ri, _)) = best {
				if let Some(spine) = rect_spines[ri] {
					if let Some(stub) = stub_to_target(spine, shaft2, params.hall_width) {
						hall_plans.push(stub);
					}
				}
			}
			let _ = si;
		}

		// Inter-rect portal corridors (geometry only — no internal wall passages yet).
		for i in 0..primary.len() {
			for j in (i + 1)..primary.len() {
				let Some(portal) =
					shared_edge_portal(primary[i].to_aabb2(), primary[j].to_aabb2(), params.portal_width)
				else {
					continue;
				};
				let Some(si) = rect_spines[i] else { continue };
				let Some(sj) = rect_spines[j] else { continue };
				hall_plans.push(portal);
				if let Some(stub) = stub_to_target(si, portal, params.hall_width) {
					hall_plans.push(stub);
				}
				if let Some(stub) = stub_to_target(sj, portal, params.hall_width) {
					hall_plans.push(stub);
				}
			}
		}

		hall_plans = merge_overlapping(hall_plans);

		// --- room cells: subtract halls + shafts from each primary rect ---
		let mut cells = Vec::new();
		let mut next_id = 0_u32;
		for rect in &primary {
			let host = rect.to_aabb2();
			let mut cuts: Vec<Aabb2d> = hall_plans
				.iter()
				.filter(|h| aabb2_intersects(host, **h))
				.copied()
				.collect();
			cuts.extend(
				shaft_excludes
					.iter()
					.filter(|s| aabb2_intersects(host, **s))
					.copied(),
			);
			for rem in subtract_aabb2(host, &cuts) {
				let id = next_id;
				next_id += 1;
				cells.push(PlanCell::new(id, rem));
			}
		}

		let cells = split_toward_min_room(&cells, params.min_room_size, &mut next_id);

		// --- apartment groups (allocation only — no piece shells) ---
		let group_ids = group_cells_to_apartments(
			&cells,
			&hall_plans,
			params.min_room_size,
			params.target_apartment_area,
		);
		let cell_by_id: std::collections::HashMap<u32, PlanCell> =
			cells.iter().map(|c| (c.id, *c)).collect();
		let mut used_cells = std::collections::HashSet::new();
		let mut apartment_groups = Vec::new();
		for (gi, ids) in group_ids.into_iter().enumerate() {
			let mut pieces = Vec::new();
			for cid in &ids {
				used_cells.insert(*cid);
				let Some(cell) = cell_by_id.get(cid) else {
					continue;
				};
				let bounds = plan_to_aabb3(&confines.bounds, cell.bounds, PlanAxes::XZ);
				pieces.push(Confines::new(bounds, confines.roll, Openings::new()));
			}
			if pieces.is_empty() {
				continue;
			}
			apartment_groups.push(ApartmentGroup {
				group_id: gi as u32,
				cell_ids: ids,
				pieces,
			});
		}

		// --- janitorial slots (allocation only) ---
		let mut janitorial_slots = Vec::new();
		let jan_side = params.janitorial_side;
		for cell in &cells {
			if used_cells.contains(&cell.id) {
				continue;
			}
			if !cell_has_hall_frontage(cell, &hall_plans, EPS) {
				continue;
			}
			let size = cell.size();
			if size.x + EPS < jan_side || size.y + EPS < jan_side {
				continue;
			}
			let pocket = janitorial_pocket(cell.bounds, &hall_plans, jan_side);
			let Some(pocket) = pocket else { continue };
			let bounds = plan_to_aabb3(&confines.bounds, pocket, PlanAxes::XZ);
			let slot = Confines::new(bounds, confines.roll, Openings::new());
			if Janitorial::from_confines(&slot).is_ok() {
				janitorial_slots.push(slot);
				used_cells.insert(cell.id);
			}
		}

		let hall_bounds: Vec<Aabb3d> = hall_plans
			.iter()
			.map(|h| plan_to_aabb3(&confines.bounds, *h, PlanAxes::XZ))
			.collect();

		// Shell: walls + facade windows only. No solid IFloor slab — halls/shafts
		// stay open; allocated rooms get thin region floors below.
		let edges = ifloor_params.wall_edges();
		let mut shell_openings = Openings::new();
		shell_openings.extend(&facade_apertures(&edges, height));

		let shell = IFloor::new(IFloorParams {
			openings: shell_openings.clone(),
			floor: IFloorSlab::None,
			..ifloor_params
		});

		let mut region_floors = Vec::new();
		for group in &apartment_groups {
			for piece in &group.pieces {
				if let Some(f) = region_floor_slab(piece, y0) {
					region_floors.push(f);
				}
			}
		}
		for slot in &janitorial_slots {
			if let Some(f) = region_floor_slab(slot, y0) {
				region_floors.push(f);
			}
		}

		// Plan openings = shell openings + shaft bookkeeping for tooling.
		let mut openings = shell_openings;
		for (i, shaft) in shaft_bounds.iter().enumerate() {
			let slot = shaft_slots[i];
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft", slot.to_string()),
				Opening::new(*shaft, OpeningLabel::Shaft),
			);
			for id in &shaft_inbound[i] {
				openings.insert(id.clone(), Opening::new(*shaft, OpeningLabel::Shaft));
			}
		}

		let plan = Self {
			parameterized: params,
			center_xz,
			storey_height: height,
			roll: confines.roll,
			openings,
			shell,
			hall_bounds,
			shaft_bounds,
			shaft_slots,
			shaft_inbound,
			apartment_groups,
			janitorial_slots,
			region_floors,
		};
		let regions = plan.fillable_regions();
		Ok((plan, regions))
	}

	pub fn apartment_groups(&self) -> &[ApartmentGroup] {
		&self.apartment_groups
	}

	pub fn janitorial_slots(&self) -> &[Confines] {
		&self.janitorial_slots
	}

	/// Residual halls / shafts / apartment pieces / janitorial + stack region.
	pub fn fillable_regions(&self) -> FillableRegions {
		let mut within = Vec::new();

		for hall in &self.hall_bounds {
			within.push(FillRegion::new(
				SpaceKind::Hallway,
				Confines::new(*hall, self.roll, Openings::new()),
			));
		}

		for (i, shaft) in self.shaft_bounds.iter().enumerate() {
			let slot = self.shaft_slots.get(i).copied().unwrap_or(i);
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft", slot.to_string()),
				Opening::new(*shaft, OpeningLabel::Shaft),
			);
			if let Some(ids) = self.shaft_inbound.get(i) {
				for id in ids {
					openings.insert(id.clone(), Opening::new(*shaft, OpeningLabel::Shaft));
				}
			}
			within.push(FillRegion::new(
				SpaceKind::InternalSpace,
				Confines::new(*shaft, self.roll, openings),
			));
		}

		for group in &self.apartment_groups {
			for piece in &group.pieces {
				within.push(FillRegion::new(
					SpaceKind::Custom(format!("apartment:{}", group.group_id)),
					piece.clone(),
				));
			}
		}

		for slot in &self.janitorial_slots {
			within.push(FillRegion::new(
				SpaceKind::Custom("janitorial".into()),
				slot.clone(),
			));
		}

		let atop_bounds = primary_union_aabb2(&self.shell.plan_rects()).unwrap_or(Aabb2d {
			min: Vec2::new(self.center_xz.x - 1.0, self.center_xz.z - 1.0),
			max: Vec2::new(self.center_xz.x + 1.0, self.center_xz.z + 1.0),
		});
		let atop = vec![StackRegion {
			bounds: atop_bounds,
			height: self.storey_height,
			roll: self.roll,
			openings: self.openings.clone(),
		}];

		FillableRegions { within, atop }
	}

	/// Demo inbound shafts for every fixed I-skeleton slot.
	pub fn shaft_requests_for_all_slots(
		params: &IApartmentParameterized,
		confines: &Confines,
	) -> Openings {
		let height = (confines.bounds.max.y - confines.bounds.min.y).max(0.0);
		let y0 = confines.bounds.min.y;
		let center = confines.center();
		let center_xz = Vec3::new(center.x, y0, center.z);
		let Ok(ifloor_params) =
			derive_ifloor_params(params, confines, center_xz, height, IFloorSlab::None)
		else {
			return Openings::new();
		};
		let mut openings = Openings::new();
		for (slot, shaft) in skeleton_shaft_candidates(&ifloor_params, height, y0)
			.iter()
			.enumerate()
		{
			let mid = Vec3::from((shaft.min + shaft.max) * 0.5);
			let half = 0.35_f32;
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft_req", slot.to_string()),
				Opening::new(
					Aabb3d::from_min_max(
						Vec3::new(mid.x - half, y0, mid.z - half),
						Vec3::new(mid.x + half, y0 + height.min(3.0), mid.z + half),
					),
					OpeningLabel::Shaft,
				),
			);
		}
		openings
	}
}

impl Fit for IApartmentFloorPlan {
	fn fit_to_confines(
		confines: &Confines,
		noise: procedural_common::NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = IApartmentParameterized::sample(confines, noise)?;
		Self::from_parameterized(params, confines)
	}
}

impl BuildingComponents for IApartmentFloorPlan {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.shell.panel_nodes_for_level(level);
		for f in &self.region_floors {
			out.extend(f.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.shell.joint_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for (i, hall) in self.hall_bounds.iter().enumerate() {
			out.push_free(label_filling_aabb(
				LabelStyle::Cyan,
				&format!("Hallway {}", i + 1),
				hall,
				self.roll,
			));
		}
		for (i, shaft) in self.shaft_bounds.iter().enumerate() {
			out.push_free(label_filling_aabb(
				LabelStyle::Magenta,
				&format!("Shaft {}", i + 1),
				shaft,
				self.roll,
			));
		}
		for group in &self.apartment_groups {
			for (pi, piece) in group.pieces.iter().enumerate() {
				let text = if group.pieces.len() == 1 {
					format!("Apt {}", group.group_id + 1)
				} else {
					format!("Apt {} · {}", group.group_id + 1, pi + 1)
				};
				out.push_free(label_filling_aabb(
					LabelStyle::Blue,
					&text,
					&piece.bounds,
					self.roll,
				));
			}
		}
		for (i, slot) in self.janitorial_slots.iter().enumerate() {
			out.push_free(label_filling_aabb(
				LabelStyle::Gray,
				&format!("Janitorial {}", i + 1),
				&slot.bounds,
				self.roll,
			));
		}
		out
	}
}

fn derive_ifloor_params(
	params: &IApartmentParameterized,
	confines: &Confines,
	center_xz: Vec3,
	height: f32,
	ceiling: IFloorSlab,
) -> Result<IFloorParams, FitError> {
	let footprint = aabb_xz_extent(&confines.bounds);
	let short = footprint.x.min(footprint.y);
	let stem_w = (short * params.stem_width_frac).max(super::parameterized::MIN_STEM_WIDTH);
	if footprint.y < 2.0 * stem_w + 2.0 {
		return Err(FitError::TooSmall { reason: "stem_depth" });
	}
	let central_depth = footprint.y - 2.0 * stem_w;
	let side = ((footprint.x - stem_w) * 0.5).max(0.0);
	Ok(IFloorParams {
		center_xz,
		top_left_length: Some(side),
		top_right_length: Some(side),
		central_rectangle: Vec2::new(stem_w, central_depth),
		bottom_left_length: Some(side),
		bottom_right_length: Some(side),
		storey_height: height,
		openings: Openings::new(),
		floor: IFloorSlab::None, // region floors authored separately so halls stay open
		ceiling,
		style: PanelStyle::RoughStonework,
		joint_thickness: crate::paneling::DEFAULT_PANEL_THICKNESS,
	})
}

/// Fixed shaft pockets at the I's reentrant corners (stem × flange junctions).
///
/// Order: top-west, top-east, bottom-west, bottom-east (slots omitted when that
/// flange bar is absent).
fn skeleton_shaft_candidates(params: &IFloorParams, height: f32, y0: f32) -> Vec<Aabb3d> {
	let cx = params.center_xz.x;
	let cz = params.center_xz.z;
	let w = params.central_rectangle.x.max(EPS);
	let d = params.central_rectangle.y.max(EPS);
	let half_w = w * 0.5;
	let half_d = d * 0.5;
	let stem_x0 = cx - half_w;
	let stem_x1 = cx + half_w;
	let stem_z0 = cz - half_d;
	let stem_z1 = cz + half_d;
	let has_top = params.top_left_length.is_some() || params.top_right_length.is_some();
	let has_bot = params.bottom_left_length.is_some() || params.bottom_right_length.is_some();
	let side = SHAFT_SIDE.min(w * 0.9).min(d * 0.45).max(1.6);
	let mut out = Vec::new();
	if has_top {
		// Into the stem below the top flange junction.
		out.push(corner_shaft(stem_x0, stem_z1 - side, side, y0, height)); // TW
		out.push(corner_shaft(stem_x1 - side, stem_z1 - side, side, y0, height)); // TE
	}
	if has_bot {
		out.push(corner_shaft(stem_x0, stem_z0, side, y0, height)); // BW
		out.push(corner_shaft(stem_x1 - side, stem_z0, side, y0, height)); // BE
	}
	out
}

fn corner_shaft(x0: f32, z0: f32, side: f32, y0: f32, height: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(x0, y0, z0),
		Vec3::new(x0 + side, y0 + height, z0 + side),
	)
}

fn map_inbound_shafts(
	openings: &mut Openings,
	candidates: &[Aabb3d],
) -> Vec<Vec<OpeningId>> {
	if candidates.is_empty() {
		return Vec::new();
	}
	let regions: Vec<Aabb2d> = candidates
		.iter()
		.map(|c| {
			// Voronoi-ish claim: nearest candidate center, with a generous region.
			let p = aabb3_to_plan(c, PlanAxes::XZ);
			let pad = SHAFT_SIDE * 2.0;
			Aabb2d {
				min: p.min - Vec2::splat(pad),
				max: p.max + Vec2::splat(pad),
			}
		})
		.collect();
	let mut inbound: Vec<Vec<OpeningId>> = (0..candidates.len()).map(|_| Vec::new()).collect();
	let shaft_ids: Vec<OpeningId> = openings
		.iter()
		.filter(|(_, o)| matches!(o.label, OpeningLabel::Shaft))
		.map(|(id, _)| id.clone())
		.collect();
	for id in shaft_ids {
		let Some(opening) = openings.openings.get_mut(&id) else {
			continue;
		};
		let Some(slot) = best_slot(&opening.bounds, &regions) else {
			continue;
		};
		opening.bounds = candidates[slot];
		inbound[slot].push(id);
	}
	inbound
}

fn best_slot(bounds: &Aabb3d, regions: &[Aabb2d]) -> Option<usize> {
	let mut best: Option<(usize, f32, f32)> = None;
	let c = Vec3::from((bounds.min + bounds.max) * 0.5);
	for (i, region) in regions.iter().enumerate() {
		let overlap = aabb_xz_overlap_area(bounds, region);
		let rc = 0.5 * (region.min + region.max);
		let dist = (c.x - rc.x).hypot(c.z - rc.y);
		match best {
			None => best = Some((i, overlap, dist)),
			Some((_, bo, bd)) => {
				if overlap > bo + EPS || ((overlap - bo).abs() <= EPS && dist < bd) {
					best = Some((i, overlap, dist));
				}
			}
		}
	}
	best.map(|(i, _, _)| i)
}

fn spine_band(host: Aabb2d, hall_width: f32, offset: f32) -> Option<Aabb2d> {
	let size = host.max - host.min;
	if size.x < hall_width * 2.0 && size.y < hall_width * 2.0 {
		return None;
	}
	let hw = hall_width.max(EPS);
	let along_x = size.x >= size.y;
	if along_x {
		if size.y + EPS < hw {
			return None;
		}
		let span = (size.y - hw).max(0.0);
		let t = (offset + 0.5).clamp(0.0, 1.0);
		let z0 = host.min.y + span * t;
		Some(Aabb2d {
			min: Vec2::new(host.min.x, z0),
			max: Vec2::new(host.max.x, z0 + hw),
		})
	} else {
		if size.x + EPS < hw {
			return None;
		}
		let span = (size.x - hw).max(0.0);
		let t = (offset + 0.5).clamp(0.0, 1.0);
		let x0 = host.min.x + span * t;
		Some(Aabb2d {
			min: Vec2::new(x0, host.min.y),
			max: Vec2::new(x0 + hw, host.max.y),
		})
	}
}

fn stub_to_target(spine: Aabb2d, target: Aabb2d, hall_width: f32) -> Option<Aabb2d> {
	if aabb2_intersects(spine, target) {
		return None;
	}
	let sc = 0.5 * (spine.min + spine.max);
	let tc = 0.5 * (target.min + target.max);
	let hw = hall_width.max(EPS);
	let dx = tc.x - sc.x;
	let dz = tc.y - sc.y;
	if dx.abs() >= dz.abs() {
		let z0 = (sc.y - hw * 0.5)
			.max(spine.min.y.min(target.min.y))
			.min(spine.max.y.max(target.max.y) - hw);
		let x0 = sc.x.min(tc.x);
		let x1 = sc.x.max(tc.x);
		if x1 - x0 <= EPS {
			return None;
		}
		Some(Aabb2d {
			min: Vec2::new(x0, z0),
			max: Vec2::new(x1, z0 + hw),
		})
	} else {
		let x0 = (sc.x - hw * 0.5)
			.max(spine.min.x.min(target.min.x))
			.min(spine.max.x.max(target.max.x) - hw);
		let z0 = sc.y.min(tc.y);
		let z1 = sc.y.max(tc.y);
		if z1 - z0 <= EPS {
			return None;
		}
		Some(Aabb2d {
			min: Vec2::new(x0, z0),
			max: Vec2::new(x0 + hw, z1),
		})
	}
}

fn shared_edge_portal(a: Aabb2d, b: Aabb2d, portal_width: f32) -> Option<Aabb2d> {
	let pw = portal_width.max(EPS);
	let touch_x = (a.max.x - b.min.x).abs() <= EPS || (b.max.x - a.min.x).abs() <= EPS;
	let z0 = a.min.y.max(b.min.y);
	let z1 = a.max.y.min(b.max.y);
	if touch_x && z1 - z0 > pw {
		let x = if (a.max.x - b.min.x).abs() <= EPS {
			a.max.x
		} else {
			b.max.x
		};
		let mid = 0.5 * (z0 + z1);
		return Some(Aabb2d {
			min: Vec2::new(x - pw * 0.5, mid - pw * 0.5),
			max: Vec2::new(x + pw * 0.5, mid + pw * 0.5),
		});
	}
	let touch_z = (a.max.y - b.min.y).abs() <= EPS || (b.max.y - a.min.y).abs() <= EPS;
	let x0 = a.min.x.max(b.min.x);
	let x1 = a.max.x.min(b.max.x);
	if touch_z && x1 - x0 > pw {
		let z = if (a.max.y - b.min.y).abs() <= EPS {
			a.max.y
		} else {
			b.max.y
		};
		let mid = 0.5 * (x0 + x1);
		return Some(Aabb2d {
			min: Vec2::new(mid - pw * 0.5, z - pw * 0.5),
			max: Vec2::new(mid + pw * 0.5, z + pw * 0.5),
		});
	}
	None
}

fn edge_touch(a: Aabb2d, b: Aabb2d) -> bool {
	let eps = EPS;
	let x_overlap = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)) > eps;
	let y_overlap = (a.max.y.min(b.max.y) - a.min.y.max(b.min.y)) > eps;
	let touch_x = (a.max.x - b.min.x).abs() <= eps || (b.max.x - a.min.x).abs() <= eps;
	let touch_y = (a.max.y - b.min.y).abs() <= eps || (b.max.y - a.min.y).abs() <= eps;
	(touch_x && y_overlap) || (touch_y && x_overlap)
}

fn janitorial_pocket(cell: Aabb2d, halls: &[Aabb2d], side: f32) -> Option<Aabb2d> {
	let hall = halls.iter().copied().find(|h| edge_touch(cell, *h))?;
	let s = side.max(EPS);
	if cell.max.x - cell.min.x + EPS < s || cell.max.y - cell.min.y + EPS < s {
		return None;
	}
	let from_max_x = (cell.max.x - hall.min.x).abs() <= EPS;
	let from_min_x = (hall.max.x - cell.min.x).abs() <= EPS;
	let from_max_y = (cell.max.y - hall.min.y).abs() <= EPS;
	let from_min_y = (hall.max.y - cell.min.y).abs() <= EPS;
	let (x0, y0) = if from_max_x {
		(cell.max.x - s, cell.min.y)
	} else if from_min_x {
		(cell.min.x, cell.min.y)
	} else if from_max_y {
		(cell.min.x, cell.max.y - s)
	} else if from_min_y {
		(cell.min.x, cell.min.y)
	} else {
		(cell.min.x, cell.min.y)
	};
	Some(Aabb2d {
		min: Vec2::new(x0, y0),
		max: Vec2::new(x0 + s, y0 + s),
	})
}

fn merge_overlapping(mut halls: Vec<Aabb2d>) -> Vec<Aabb2d> {
	let mut out = Vec::new();
	'outer: for h in halls.drain(..) {
		for o in &out {
			if aabb2_contains(*o, h) {
				continue 'outer;
			}
		}
		out.retain(|o| !aabb2_contains(h, *o));
		out.push(h);
	}
	out
}

fn aabb2_contains(a: Aabb2d, b: Aabb2d) -> bool {
	a.min.x <= b.min.x + EPS
		&& a.min.y <= b.min.y + EPS
		&& a.max.x + EPS >= b.max.x
		&& a.max.y + EPS >= b.max.y
}

fn aabb2_intersects(a: Aabb2d, b: Aabb2d) -> bool {
	a.min.x < b.max.x - EPS
		&& b.min.x < a.max.x - EPS
		&& a.min.y < b.max.y - EPS
		&& b.min.y < a.max.y - EPS
}

fn facade_apertures(edges: &[WallEdge], height: f32) -> Openings {
	let mut out = Openings::new();
	let win_w = 1.4_f32;
	let win_h = (height * 0.45).clamp(1.1, height.max(1.1));
	let sill = (height * 0.30).clamp(0.8, height * 0.45);
	let pitch = 3.0_f32;
	let mut wi = 0usize;
	for edge in edges {
		let len = edge.length();
		if len < 3.5 {
			continue;
		}
		let mut along = pitch * 0.5;
		while along + win_w * 0.5 < len - 0.35 {
			out.insert(
				OpeningId::scoped(SCOPE, "facade_aperture", wi.to_string()),
				IFloor::edge_aperture_opening_at(*edge, along, win_w, win_h, sill),
			);
			wi += 1;
			along += pitch;
		}
	}
	out
}

fn region_floor_slab(confines: &Confines, y0: f32) -> Option<FittedRectangle> {
	let min = Vec3::from(confines.bounds.min);
	let max = Vec3::from(confines.bounds.max);
	let footprint = Vec2::new((max.x - min.x).max(0.0), (max.z - min.z).max(0.0));
	if footprint.x < EPS || footprint.y < EPS {
		return None;
	}
	let plan = PlanRect::new(
		Vec3::new(0.5 * (min.x + max.x), y0, 0.5 * (min.z + max.z)),
		footprint.x,
		footprint.y,
	);
	let t = DEFAULT_PANEL_THICKNESS;
	Some(FittedRectangle::new(
		PanelStyle::RoughStonework,
		PanelPoint::new(plan.sw(), t),
		PanelPoint::new(plan.se(), t),
		PanelPoint::new(plan.nw(), t),
		PanelPoint::new(plan.ne(), t),
	))
}

fn primary_union_aabb2(rects: &[IFloorPlanRect]) -> Option<Aabb2d> {
	let mut iter = rects.iter();
	let first = iter.next()?;
	let mut min = Vec2::new(first.min_x, first.min_z);
	let mut max = Vec2::new(first.max_x, first.max_z);
	for r in iter {
		min.x = min.x.min(r.min_x);
		min.y = min.y.min(r.min_z);
		max.x = max.x.max(r.max_x);
		max.y = max.y.max(r.max_z);
	}
	Some(Aabb2d { min, max })
}

fn label_filling_aabb(style: LabelStyle, text: &str, aabb: &Aabb3d, yaw: f32) -> LabelNode {
	let center = Vec3::from(aabb.center());
	let extents = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	LabelNode::rectangle(style, text, center, extents, yaw)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::MapsOpenings;
	use lod::gen::LodSceneLevel;
	use procedural_common::NoiseParams;
	use richmond_building_components::BuildingComponents;

	fn large_confines(openings: Openings) -> Confines {
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(-22.0, 0.0, -18.0), Vec3::new(22.0, 3.5, 18.0)),
			0.0,
			openings,
		)
	}

	#[test]
	fn packs_halls_and_apartment_groups() {
		let empty = large_confines(Openings::new());
		let params = IApartmentParameterized::sample(&empty, NoiseParams::default()).unwrap();
		let openings = IApartmentFloorPlan::shaft_requests_for_all_slots(&params, &empty);
		let confines = large_confines(openings);
		let (plan, regions) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		assert!(!plan.hall_bounds.is_empty());
		assert!(!plan.apartment_groups.is_empty());
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::Hallway));
		assert!(!plan.shell.plan_rects().is_empty());
	}

	#[test]
	fn shafts_are_few_fixed_skeleton_slots() {
		let empty = large_confines(Openings::new());
		let params = IApartmentParameterized::sample(&empty, NoiseParams::default()).unwrap();
		let openings = IApartmentFloorPlan::shaft_requests_for_all_slots(&params, &empty);
		// Full I → at most 4 skeleton slots.
		assert!(openings.openings.len() <= 4);
		assert!(!openings.openings.is_empty());
		let confines = large_confines(openings);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		assert!(plan.shaft_bounds.len() <= 4);
		assert!(!plan.shaft_bounds.is_empty());
	}

	#[test]
	fn facade_apertures_map_and_hall_cuts_exist() {
		let empty = large_confines(Openings::new());
		let params = IApartmentParameterized::sample(&empty, NoiseParams::default()).unwrap();
		let openings = IApartmentFloorPlan::shaft_requests_for_all_slots(&params, &empty);
		let confines = large_confines(openings);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		let facade: Vec<_> = plan
			.openings
			.iter()
			.filter(|(id, o)| {
				id.as_str().contains("facade_aperture")
					&& matches!(o.label, OpeningLabel::Aperture)
			})
			.collect();
		assert!(!facade.is_empty(), "expected facade apertures");
		assert!(
			facade
				.iter()
				.any(|(id, _)| plan.shell.mapped_opening(id).is_some()),
			"expected at least one mapped facade aperture on the IFloor"
		);
		assert!(!plan.region_floors.is_empty(), "allocated rooms should have floors");
		assert!(!plan.hall_bounds.is_empty(), "halls allocated");
		assert!(!plan
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
		assert!(!plan
			.panel_nodes_for_level(LodSceneLevel::High)
			.is_empty());
	}
}
