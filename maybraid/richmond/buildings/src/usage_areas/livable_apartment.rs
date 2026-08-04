//! Livable apartment: entryway → common / private zones → living quarters.
//!
//! Layout (first cut):
//! 1. Carve an **entryway** box at the hall door.
//! 2. Split the remainder into **common** (living / kitchen / dining / sitting)
//!    and **private** (bedroom / bath / study) zones by area share.
//! 3. Map total m² to a room-count program; guillotine-split each zone toward
//!    those targets; first-fit the matching quarter type.
//! 4. Leftovers stay [`SpaceKind::InternalSpace`] residuals, or become a
//!    household closet when small enough. [`crate::IApartmentFullStorey`] maps
//!    leftover InternalSpace → [`SpaceKind::ClosetSpace`].

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::{aabb2_area, Aabb2dPack, NoiseConfig, NoiseParams};
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::{LabelNode, LabelStyle};
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{
	Confines, FillRegion, FillableRegions, Fit, FitError, MultiConfines, SpaceKind,
};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::rect_fit::RectInset;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::shells::ortho::{standing_face_opening, WallEdge};
use crate::shells::{RectFloor, RectFloorParams, RectFloorSlab};
use crate::usage_areas::common_bedroom::CommonBedroom;
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::livable_quarters::{
	DiningRoom, Kitchen, LivingRoom, ResidentialBathroom, ResidentialHalfBathroom, SittingRoom,
	Study,
};
use crate::usage_areas::plan_cells::{cells_edge_adjacent, subtract_aabb2, PlanCell};

const EPS: f32 = 1e-3;
const DOOR_WIDTH: f32 = 1.0;
const ENTRY_DEPTH: f32 = 1.8;
const ENTRY_WIDTH: f32 = 2.2;
const MIN_ROOM: f32 = 2.2;
const SCOPE: &str = "livable_apartment";

/// One packed space inside an apartment.
#[derive(Debug, Clone, PartialEq)]
pub enum ApartmentRoom {
	Entryway {
		label: LabelNode,
		confines: Confines,
	},
	HouseholdCloset {
		label: LabelNode,
		confines: Confines,
	},
	Bedroom(CommonBedroom),
	Living(LivingRoom),
	Kitchen(Kitchen),
	Dining(DiningRoom),
	Bathroom(ResidentialBathroom),
	HalfBath(ResidentialHalfBathroom),
	Sitting(SittingRoom),
	Study(Study),
}

impl ApartmentRoom {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Entryway { .. } | Self::HouseholdCloset { .. } => Layers::new(),
			Self::Bedroom(r) => r.panel_nodes_for_level(level),
			Self::Living(r) => r.panel_nodes_for_level(level),
			Self::Kitchen(r) => r.panel_nodes_for_level(level),
			Self::Dining(r) => r.panel_nodes_for_level(level),
			Self::Bathroom(r) => r.panel_nodes_for_level(level),
			Self::HalfBath(r) => r.panel_nodes_for_level(level),
			Self::Sitting(r) => r.panel_nodes_for_level(level),
			Self::Study(r) => r.panel_nodes_for_level(level),
		}
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		match self {
			Self::Entryway { .. } | Self::HouseholdCloset { .. } => Layers::new(),
			Self::Bedroom(r) => r.joint_nodes_for_level(level),
			Self::Living(r) => r.joint_nodes_for_level(level),
			Self::Kitchen(r) => r.joint_nodes_for_level(level),
			Self::Dining(r) => r.joint_nodes_for_level(level),
			Self::Bathroom(r) => r.joint_nodes_for_level(level),
			Self::HalfBath(r) => r.joint_nodes_for_level(level),
			Self::Sitting(r) => r.joint_nodes_for_level(level),
			Self::Study(r) => r.joint_nodes_for_level(level),
		}
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		match self {
			Self::Entryway { label, .. } | Self::HouseholdCloset { label, .. } => {
				let mut out = Layers::new();
				out.push_free(label.clone());
				out
			}
			Self::Bedroom(r) => r.label_nodes_for_level(level),
			Self::Living(r) => r.label_nodes_for_level(level),
			Self::Kitchen(r) => r.label_nodes_for_level(level),
			Self::Dining(r) => r.label_nodes_for_level(level),
			Self::Bathroom(r) => r.label_nodes_for_level(level),
			Self::HalfBath(r) => r.label_nodes_for_level(level),
			Self::Sitting(r) => r.label_nodes_for_level(level),
			Self::Study(r) => r.label_nodes_for_level(level),
		}
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		match self {
			Self::Entryway { .. } | Self::HouseholdCloset { .. } => Layers::new(),
			Self::Bedroom(r) => r.furniture_nodes_for_level(level),
			Self::Living(r) => r.furniture_nodes_for_level(level),
			Self::Kitchen(r) => r.furniture_nodes_for_level(level),
			Self::Dining(r) => r.furniture_nodes_for_level(level),
			Self::Bathroom(r) => r.furniture_nodes_for_level(level),
			Self::HalfBath(r) => r.furniture_nodes_for_level(level),
			Self::Sitting(r) => r.furniture_nodes_for_level(level),
			Self::Study(r) => r.furniture_nodes_for_level(level),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuarterKind {
	Bedroom,
	Living,
	Kitchen,
	Dining,
	Bathroom,
	HalfBath,
	Sitting,
	Study,
}

#[derive(Debug, Clone, Copy)]
struct ProgramCounts {
	bedrooms: u8,
	bathrooms: u8,
	half_baths: u8,
	kitchens: u8,
	dining: u8,
	living: u8,
	sitting: u8,
	studies: u8,
}

/// One apartment group: envelope + entryway / common / private quarters.
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartment {
	pub region_id: u32,
	/// Envelope cells that make up this apartment.
	pub cells: MultiConfines,
	/// Packed spaces (entryway, quarters, household closets).
	pub rooms: Vec<ApartmentRoom>,
	/// Partition strips between packed rooms (with connecting passages).
	pub partitions: Vec<ClippedRectangularStrip>,
	/// Optional envelope shell for the primary / first cell (presentation).
	pub shell: Option<RectFloor>,
}

impl LivableApartment {
	pub fn from_confines(
		region_id: u32,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_multi(
			region_id,
			&MultiConfines::new([FillRegion::new(SpaceKind::InternalSpace, confines.clone())]),
			noise,
		)
	}

	pub fn from_multi(
		region_id: u32,
		cells: &MultiConfines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		if cells.is_empty() {
			return Err(FitError::TooSmall {
				reason: "livable_empty",
			});
		}
		for part in cells.iter() {
			let fp = part.confines.footprint();
			let height =
				(part.confines.bounds.max.y - part.confines.bounds.min.y).max(0.0);
			if fp.x < 2.0 || fp.y < 2.0 {
				return Err(FitError::TooSmall {
					reason: "livable_footprint",
				});
			}
			if height < 2.0 {
				return Err(FitError::TooSmall {
					reason: "livable_height",
				});
			}
		}

		let y0 = Vec3::from(cells.parts[0].confines.bounds.min).y;
		let y1 = Vec3::from(cells.parts[0].confines.bounds.max).y;
		let roll = cells.parts[0].confines.roll;
		let total_area: f32 = cells
			.iter()
			.map(|p| {
				let fp = p.confines.footprint();
				fp.x * fp.y
			})
			.sum();
		let program = program_from_area(total_area, noise, cells.parts[0].confines.center());

		let mut rooms = Vec::new();
		let mut residual_within = Vec::new();
		let mut filled_confines: Vec<Confines> = Vec::new();

		// --- Entryway on the door cell ------------------------------------
		let (door_ci, mut work_rects) = collect_work_rects(cells);
		let door_cell = host_xz(&cells.parts[door_ci].confines.bounds);
		let door_opening = find_entry_door(&cells.parts[door_ci].confines.openings)
			.or_else(|| find_entry_door(&cells.parts[0].confines.openings));

		if let Some((_id, door)) = door_opening {
			if let Some((entry, rem)) = carve_entryway(door_cell, &door, ENTRY_DEPTH, ENTRY_WIDTH)
			{
				let entry_c = confines_from_xz(entry, y0, y1, roll, &Openings::new());
				rooms.push(ApartmentRoom::Entryway {
					label: label_filling_aabb(
						LabelStyle::Cyan,
						"Entryway",
						&entry_c.bounds,
						roll,
					),
					confines: entry_c.clone(),
				});
				filled_confines.push(entry_c);
				// Replace door cell in work_rects with carve remainders.
				work_rects.retain(|r| aabb2_area(*r) > EPS * EPS);
				// Drop the original door-cell footprint if still present.
				work_rects.retain(|r| !aabb2_near_eq(*r, door_cell));
				work_rects.extend(rem);
			}
		}

		work_rects.retain(|r| {
			let s = r.max - r.min;
			s.x + EPS >= MIN_ROOM && s.y + EPS >= MIN_ROOM && aabb2_area(*r) > 4.0
		});
		if work_rects.is_empty() {
			// Degenerate after entry — still accept if we at least have entryway.
			if rooms.is_empty() {
				return Err(FitError::TooSmall {
					reason: "livable_no_body",
				});
			}
			let shell = cells.parts.first().and_then(|p| try_shell(&p.confines));
			return Ok((
				Self {
					region_id,
					cells: cells.clone(),
					rooms,
					partitions: Vec::new(),
					shell,
				},
				FillableRegions {
					within: residual_within,
					atop: Vec::new(),
				},
			));
		}

		// --- Common / private split + greedy quarter packing --------------
		// Private rooms (bedrooms) need larger contiguous slots; allocate them first.
		let body_area: f32 = work_rects.iter().map(|r| aabb2_area(*r)).sum();
		let private_frac = if program.bedrooms == 0 { 0.25 } else { 0.55 };
		let private_target = body_area * private_frac;
		let (mut private_free, mut common_free) =
			allocate_zone_rects(&work_rects, private_target);

		pack_kinds_into_zone(
			&mut private_free,
			&private_kind_list(program),
			y0,
			y1,
			roll,
			noise,
			&mut rooms,
			&mut filled_confines,
			&mut residual_within,
		)?;
		pack_kinds_into_zone(
			&mut common_free,
			&common_kind_list(program),
			y0,
			y1,
			roll,
			noise,
			&mut rooms,
			&mut filled_confines,
			&mut residual_within,
		)?;

		for scrap in common_free.into_iter().chain(private_free) {
			let confines = confines_from_xz(scrap, y0, y1, roll, &Openings::new());
			push_leftover(&mut rooms, &mut residual_within, &mut filled_confines, confines);
		}

		if rooms.is_empty() {
			return Err(FitError::TooSmall {
				reason: "livable_no_quarters",
			});
		}

		let filled_cells: Vec<(usize, Confines)> = filled_confines
			.iter()
			.enumerate()
			.map(|(i, c)| (i, c.clone()))
			.collect();
		let (partitions, _) = connect_filled_cells(&filled_cells, region_id);

		let shell = cells.parts.first().and_then(|p| try_shell(&p.confines));
		Ok((
			Self {
				region_id,
				cells: cells.clone(),
				rooms,
				partitions,
				shell,
			},
			FillableRegions {
				within: residual_within,
				atop: Vec::new(),
			},
		))
	}

	pub fn primary_confines(&self) -> &Confines {
		&self.cells.parts[0].confines
	}
}

fn program_from_area(area: f32, noise: NoiseParams, center: Vec3) -> ProgramCounts {
	let cfg = NoiseConfig::new(noise);
	let jitter = cfg.sample_range_f32_4d(0.0, 1.0, center.x, center.y, center.z, 44.0);
	// Counts are aspirational; greedy packing skips what will not fit.
	if area < 36.0 {
		ProgramCounts {
			bedrooms: 0,
			bathrooms: 1,
			half_baths: 0,
			kitchens: if jitter > 0.4 { 1 } else { 0 },
			dining: 0,
			living: 1,
			sitting: 0,
			studies: 0,
		}
	} else if area < 58.0 {
		ProgramCounts {
			bedrooms: 1,
			bathrooms: 1,
			half_baths: 0,
			kitchens: 1,
			dining: if jitter > 0.6 { 1 } else { 0 },
			living: 1,
			sitting: 0,
			studies: 0,
		}
	} else if area < 95.0 {
		ProgramCounts {
			bedrooms: 2,
			bathrooms: 1,
			half_baths: if jitter > 0.55 { 1 } else { 0 },
			kitchens: 1,
			dining: 1,
			living: 1,
			sitting: if jitter > 0.7 { 1 } else { 0 },
			studies: 0,
		}
	} else {
		ProgramCounts {
			bedrooms: if area > 120.0 { 3 } else { 2 },
			bathrooms: if area > 110.0 { 2 } else { 1 },
			half_baths: if jitter > 0.45 { 1 } else { 0 },
			kitchens: 1,
			dining: 1,
			living: 1,
			sitting: if jitter > 0.5 { 1 } else { 0 },
			studies: if jitter > 0.55 { 1 } else { 0 },
		}
	}
}

fn common_kind_list(p: ProgramCounts) -> Vec<QuarterKind> {
	let mut out = Vec::new();
	for _ in 0..p.living {
		out.push(QuarterKind::Living);
	}
	for _ in 0..p.kitchens {
		out.push(QuarterKind::Kitchen);
	}
	for _ in 0..p.dining {
		out.push(QuarterKind::Dining);
	}
	for _ in 0..p.sitting {
		out.push(QuarterKind::Sitting);
	}
	if out.is_empty() {
		out.push(QuarterKind::Living);
	}
	out
}

fn private_kind_list(p: ProgramCounts) -> Vec<QuarterKind> {
	let mut out = Vec::new();
	for _ in 0..p.bedrooms {
		out.push(QuarterKind::Bedroom);
	}
	for _ in 0..p.bathrooms {
		out.push(QuarterKind::Bathroom);
	}
	for _ in 0..p.half_baths {
		out.push(QuarterKind::HalfBath);
	}
	for _ in 0..p.studies {
		out.push(QuarterKind::Study);
	}
	out
}

fn collect_work_rects(cells: &MultiConfines) -> (usize, Vec<Aabb2d>) {
	let mut door_ci = 0usize;
	for (i, part) in cells.iter().enumerate() {
		if find_entry_door(&part.confines.openings).is_some() {
			door_ci = i;
			break;
		}
	}
	let rects = cells
		.iter()
		.map(|p| host_xz(&p.confines.bounds))
		.collect();
	(door_ci, rects)
}

fn find_entry_door(openings: &Openings) -> Option<(OpeningId, Opening)> {
	openings
		.iter()
		.find(|(_, o)| matches!(o.label, OpeningLabel::Passage))
		.map(|(id, o)| (id.clone(), o.clone()))
}

/// Carve a shallow entry box inward from the door contact edge.
fn carve_entryway(
	host: Aabb2d,
	door: &Opening,
	depth: f32,
	width: f32,
) -> Option<(Aabb2d, Vec<Aabb2d>)> {
	let dmin = Vec3::from(door.bounds.min);
	let dmax = Vec3::from(door.bounds.max);
	let dc = Vec2::new(0.5 * (dmin.x + dmax.x), 0.5 * (dmin.z + dmax.z));
	let depth = depth.clamp(1.2, 2.8);
	let width = width.clamp(1.4, host.max.x - host.min.x - EPS).min(host.max.y - host.min.y - EPS);

	// Pick the host edge closest to the door center.
	let dist_w = (dc.x - host.min.x).abs();
	let dist_e = (dc.x - host.max.x).abs();
	let dist_s = (dc.y - host.min.y).abs();
	let dist_n = (dc.y - host.max.y).abs();
	let edge = [dist_w, dist_e, dist_s, dist_n]
		.iter()
		.enumerate()
		.min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
		.map(|(i, _)| i)?;

	let half_w = width * 0.5;
	let entry = match edge {
		0 => {
			// west (−X), inward +X
			let y0 = (dc.y - half_w).clamp(host.min.y, host.max.y - width);
			Aabb2d {
				min: Vec2::new(host.min.x, y0),
				max: Vec2::new((host.min.x + depth).min(host.max.x), y0 + width),
			}
		}
		1 => {
			let y0 = (dc.y - half_w).clamp(host.min.y, host.max.y - width);
			Aabb2d {
				min: Vec2::new((host.max.x - depth).max(host.min.x), y0),
				max: Vec2::new(host.max.x, y0 + width),
			}
		}
		2 => {
			let x0 = (dc.x - half_w).clamp(host.min.x, host.max.x - width);
			Aabb2d {
				min: Vec2::new(x0, host.min.y),
				max: Vec2::new(x0 + width, (host.min.y + depth).min(host.max.y)),
			}
		}
		_ => {
			let x0 = (dc.x - half_w).clamp(host.min.x, host.max.x - width);
			Aabb2d {
				min: Vec2::new(x0, (host.max.y - depth).max(host.min.y)),
				max: Vec2::new(x0 + width, host.max.y),
			}
		}
	};
	if aabb2_area(entry) < 1.5 {
		return None;
	}
	let rem = subtract_aabb2(host, &[entry]);
	Some((entry, rem))
}

fn allocate_zone_rects(rects: &[Aabb2d], common_target: f32) -> (Vec<Aabb2d>, Vec<Aabb2d>) {
	let mut ordered: Vec<Aabb2d> = rects.to_vec();
	ordered.sort_by(|a, b| {
		aabb2_area(*b)
			.partial_cmp(&aabb2_area(*a))
			.unwrap_or(std::cmp::Ordering::Equal)
	});
	let mut common = Vec::new();
	let mut private = Vec::new();
	let mut common_area = 0.0_f32;
	for r in ordered {
		if common_area + EPS < common_target {
			// Prefer filling common first; bipartition oversized pieces.
			let need = (common_target - common_area).max(0.0);
			if aabb2_area(r) > need + 8.0 && need > 10.0 {
				let frac = (need / aabb2_area(r)).clamp(0.2, 0.8);
				let cut_x = (r.max.x - r.min.x) >= (r.max.y - r.min.y);
				let (a, b) = r.bipartition_by_area(cut_x, true, frac);
				common.push(a);
				common_area += aabb2_area(a);
				private.push(b);
			} else {
				common.push(r);
				common_area += aabb2_area(r);
			}
		} else {
			private.push(r);
		}
	}
	if common.is_empty() && !private.is_empty() {
		common.push(private.remove(0));
	}
	(common, private)
}

/// Greedy: carve a slot for each kind from the zone free-rect pool; skip kinds
/// that will not fit. Unconsumed free rects stay in `free` for leftover handling.
fn pack_kinds_into_zone(
	free: &mut Vec<Aabb2d>,
	kinds: &[QuarterKind],
	y0: f32,
	y1: f32,
	roll: f32,
	noise: NoiseParams,
	rooms: &mut Vec<ApartmentRoom>,
	filled_confines: &mut Vec<Confines>,
	residual_within: &mut Vec<FillRegion>,
) -> Result<(), FitError> {
	for &kind in kinds {
		let Some(host_i) = pick_host_index(free, kind) else {
			continue;
		};
		let host = free.remove(host_i);
		let cell_noise = noise_for_cell(noise, rooms.len() as i32);
		let slot_id = rooms.len() as u32;
		let (slot, rem) = take_slot(host, kind);
		match try_pack_slot(slot, y0, y1, roll, cell_noise, kind, slot_id) {
			Ok((room, confines, nested)) => {
				rooms.push(room);
				filled_confines.push(confines);
				residual_within.extend(nested.within);
				return_remnants(free, rooms, residual_within, filled_confines, rem, y0, y1, roll);
			}
			Err(FitError::TooSmall { .. }) if !aabb2_near_eq(slot, host) => {
				// Carved slot was too awkward — retry the whole host.
				match try_pack_slot(host, y0, y1, roll, cell_noise, kind, slot_id) {
					Ok((room, confines, nested)) => {
						rooms.push(room);
						filled_confines.push(confines);
						residual_within.extend(nested.within);
					}
					Err(FitError::TooSmall { .. }) => free.push(host),
					Err(err) => return Err(err),
				}
			}
			Err(FitError::TooSmall { .. }) => free.push(host),
			Err(err) => return Err(err),
		}
	}
	Ok(())
}

fn try_pack_slot(
	slot: Aabb2d,
	y0: f32,
	y1: f32,
	roll: f32,
	noise: NoiseParams,
	kind: QuarterKind,
	slot_id: u32,
) -> Result<(ApartmentRoom, Confines, FillableRegions), FitError> {
	let openings = slot_passage_openings(slot, y0, y1, slot_id);
	let confines = confines_from_xz(slot, y0, y1, roll, &openings);
	let (room, nested) = pack_quarter_into_cell(&confines, noise, kind)?;
	Ok((room, confines, nested))
}

fn return_remnants(
	free: &mut Vec<Aabb2d>,
	rooms: &mut Vec<ApartmentRoom>,
	residual_within: &mut Vec<FillRegion>,
	filled_confines: &mut Vec<Confines>,
	rem: Vec<Aabb2d>,
	y0: f32,
	y1: f32,
	roll: f32,
) {
	for r in rem {
		if rect_usable(r, 4.0) {
			free.push(r);
		} else if aabb2_area(r) > EPS * EPS {
			let scrap = confines_from_xz(r, y0, y1, roll, &Openings::new());
			push_leftover(rooms, residual_within, filled_confines, scrap);
		}
	}
}

fn pick_host_index(free: &[Aabb2d], kind: QuarterKind) -> Option<usize> {
	let need = min_area_for(kind);
	let mut best: Option<(usize, f32)> = None;
	for (i, r) in free.iter().enumerate() {
		if !rect_usable(*r, need) {
			continue;
		}
		let a = aabb2_area(*r);
		if best.map(|(_, ba)| a > ba).unwrap_or(true) {
			best = Some((i, a));
		}
	}
	best.map(|(i, _)| i)
}

fn take_slot(host: Aabb2d, kind: QuarterKind) -> (Aabb2d, Vec<Aabb2d>) {
	let want = target_area_for(kind);
	let host_a = aabb2_area(host);
	if host_a < want * 1.7 {
		return (host, Vec::new());
	}
	let frac = (want / host_a).clamp(0.28, 0.65);
	let min_d = min_dim_for(kind);
	// Prefer the cut that keeps the slot closer to square.
	let candidates = [
		host.bipartition_by_area(true, true, frac),
		host.bipartition_by_area(false, true, frac),
	];
	let mut best: Option<(Aabb2d, Aabb2d, f32)> = None;
	for (slot, rest) in candidates {
		let ss = slot.max - slot.min;
		if ss.x + EPS < min_d || ss.y + EPS < min_d {
			continue;
		}
		let aspect = ss.x.max(ss.y) / ss.x.min(ss.y).max(1e-3);
		if best.map(|(_, _, a)| aspect < a).unwrap_or(true) {
			best = Some((slot, rest, aspect));
		}
	}
	if let Some((slot, rest, _)) = best {
		let mut rem = Vec::new();
		if rect_usable(rest, 3.0) || aabb2_area(rest) > 2.0 {
			rem.push(rest);
		}
		(slot, rem)
	} else {
		(host, Vec::new())
	}
}

fn rect_usable(r: Aabb2d, min_area: f32) -> bool {
	let s = r.max - r.min;
	s.x + EPS >= MIN_ROOM && s.y + EPS >= MIN_ROOM && aabb2_area(r) + EPS >= min_area
}

fn target_area_for(kind: QuarterKind) -> f32 {
	match kind {
		QuarterKind::Bedroom => 18.0,
		QuarterKind::Living => 16.0,
		QuarterKind::Kitchen => 10.0,
		QuarterKind::Dining => 10.0,
		QuarterKind::Bathroom => 6.5,
		QuarterKind::HalfBath => 3.5,
		QuarterKind::Sitting => 10.0,
		QuarterKind::Study => 9.0,
	}
}

fn min_area_for(kind: QuarterKind) -> f32 {
	match kind {
		QuarterKind::Bedroom => 12.0,
		QuarterKind::Living => 9.0,
		QuarterKind::Kitchen => 5.0,
		QuarterKind::Dining => 5.0,
		QuarterKind::Bathroom => 4.5,
		QuarterKind::HalfBath => 2.0,
		QuarterKind::Sitting => 5.0,
		QuarterKind::Study => 5.0,
	}
}

fn min_dim_for(kind: QuarterKind) -> f32 {
	match kind {
		QuarterKind::Bedroom => 3.2,
		QuarterKind::Bathroom | QuarterKind::HalfBath => 1.6,
		_ => MIN_ROOM,
	}
}

/// Authored passage on the longest cardinal edge so quarters can clear entry.
fn slot_passage_openings(xz: Aabb2d, y0: f32, y1: f32, slot_id: u32) -> Openings {
	let mut openings = Openings::new();
	let sx = xz.max.x - xz.min.x;
	let sz = xz.max.y - xz.min.y;
	let door_w = DOOR_WIDTH.min(sx.max(sz) - 0.25).clamp(0.7, 1.15);
	let half = door_w * 0.5;
	let door_h = (y1 - y0).min(2.15).max(1.9);
	let half_d = 0.12_f32;
	let bounds = if sx >= sz {
		// Prefer south (−Z / plan −Y).
		let cx = 0.5 * (xz.min.x + xz.max.x);
		let z = xz.min.y;
		Aabb3d::from_min_max(
			Vec3::new(cx - half, y0, z - half_d),
			Vec3::new(cx + half, y0 + door_h, z + half_d),
		)
	} else {
		let cz = 0.5 * (xz.min.y + xz.max.y);
		let x = xz.min.x;
		Aabb3d::from_min_max(
			Vec3::new(x - half_d, y0, cz - half),
			Vec3::new(x + half_d, y0 + door_h, cz + half),
		)
	};
	openings.insert(
		OpeningId::scoped(SCOPE, "room_door", format!("{slot_id}")),
		Opening::passage(bounds),
	);
	openings
}

fn push_leftover(
	rooms: &mut Vec<ApartmentRoom>,
	residual_within: &mut Vec<FillRegion>,
	filled_confines: &mut Vec<Confines>,
	confines: Confines,
) {
	let area = {
		let fp = confines.footprint();
		fp.x * fp.y
	};
	if (1.8..8.0).contains(&area) {
		// Household closet is a room; Full* maps other InternalSpace leftovers.
		rooms.push(ApartmentRoom::HouseholdCloset {
			label: label_filling_aabb(
				LabelStyle::Gray,
				"HouseholdCloset",
				&confines.bounds,
				confines.roll,
			),
			confines: confines.clone(),
		});
		filled_confines.push(confines);
	} else {
		residual_within.push(FillRegion::new(SpaceKind::InternalSpace, confines));
	}
}

fn noise_for_cell(noise: NoiseParams, cell: i32) -> NoiseParams {
	NoiseParams {
		seed: noise.seed.wrapping_add(cell.wrapping_mul(97)),
		..noise
	}
}

fn pack_quarter_into_cell(
	confines: &Confines,
	noise: NoiseParams,
	preferred: QuarterKind,
) -> Result<(ApartmentRoom, FillableRegions), FitError> {
	let fallbacks: &[QuarterKind] = match preferred {
		QuarterKind::Bedroom => &[QuarterKind::Bedroom, QuarterKind::Study, QuarterKind::Sitting],
		QuarterKind::Living => &[QuarterKind::Living, QuarterKind::Sitting, QuarterKind::Dining],
		QuarterKind::Kitchen => &[QuarterKind::Kitchen, QuarterKind::Dining],
		QuarterKind::Dining => &[QuarterKind::Dining, QuarterKind::Kitchen, QuarterKind::Living],
		QuarterKind::Bathroom => &[QuarterKind::Bathroom, QuarterKind::HalfBath],
		QuarterKind::HalfBath => &[QuarterKind::HalfBath, QuarterKind::Bathroom],
		QuarterKind::Sitting => &[QuarterKind::Sitting, QuarterKind::Living, QuarterKind::Study],
		QuarterKind::Study => &[QuarterKind::Study, QuarterKind::Bedroom, QuarterKind::Sitting],
	};
	let mut last_err = FitError::TooSmall {
		reason: "livable_quarter",
	};
	for &kind in fallbacks {
		match try_fit_kind(kind, confines, noise) {
			Ok(ok) => return Ok(ok),
			Err(FitError::TooSmall { .. }) => continue,
			Err(err) => {
				last_err = err;
				break;
			}
		}
	}
	Err(last_err)
}

fn try_fit_kind(
	kind: QuarterKind,
	confines: &Confines,
	noise: NoiseParams,
) -> Result<(ApartmentRoom, FillableRegions), FitError> {
	match kind {
		QuarterKind::Bedroom => CommonBedroom::fit_to_confines(confines, noise)
			.map(|(r, n)| (ApartmentRoom::Bedroom(r), n)),
		QuarterKind::Living => LivingRoom::fit_to_confines(confines, noise)
			.map(|(r, n)| (ApartmentRoom::Living(r), n)),
		QuarterKind::Kitchen => Kitchen::fit_to_confines(confines, noise)
			.map(|(r, n)| (ApartmentRoom::Kitchen(r), n)),
		QuarterKind::Dining => DiningRoom::fit_to_confines(confines, noise)
			.map(|(r, n)| (ApartmentRoom::Dining(r), n)),
		QuarterKind::Bathroom => ResidentialBathroom::fit_to_confines(confines, noise)
			.map(|(r, n)| (ApartmentRoom::Bathroom(r), n)),
		QuarterKind::HalfBath => ResidentialHalfBathroom::fit_to_confines(confines, noise)
			.map(|(r, n)| (ApartmentRoom::HalfBath(r), n)),
		QuarterKind::Sitting => SittingRoom::fit_to_confines(confines, noise)
			.map(|(r, n)| (ApartmentRoom::Sitting(r), n)),
		QuarterKind::Study => Study::fit_to_confines(confines, noise)
			.map(|(r, n)| (ApartmentRoom::Study(r), n)),
	}
}

fn host_xz(bounds: &Aabb3d) -> Aabb2d {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	Aabb2d {
		min: Vec2::new(min.x, min.z),
		max: Vec2::new(max.x, max.z),
	}
}

fn confines_from_xz(xz: Aabb2d, y0: f32, y1: f32, roll: f32, openings: &Openings) -> Confines {
	Confines::new(
		Aabb3d::from_min_max(
			Vec3::new(xz.min.x, y0, xz.min.y),
			Vec3::new(xz.max.x, y1, xz.max.y),
		),
		roll,
		openings.clone(),
	)
}

fn aabb2_near_eq(a: Aabb2d, b: Aabb2d) -> bool {
	(a.min.x - b.min.x).abs() < 0.05
		&& (a.min.y - b.min.y).abs() < 0.05
		&& (a.max.x - b.max.x).abs() < 0.05
		&& (a.max.y - b.max.y).abs() < 0.05
}

fn connect_filled_cells(
	filled: &[(usize, Confines)],
	apartment_id: u32,
) -> (Vec<ClippedRectangularStrip>, Openings) {
	let thickness = DEFAULT_PANEL_THICKNESS.max(0.12);
	let mut partitions = Vec::new();
	let mut openings = Openings::new();
	let cells: Vec<PlanCell> = filled
		.iter()
		.map(|(ci, c)| {
			let min = Vec3::from(c.bounds.min);
			let max = Vec3::from(c.bounds.max);
			PlanCell::new(
				*ci as u32,
				Aabb2d {
					min: Vec2::new(min.x, min.z),
					max: Vec2::new(max.x, max.z),
				},
			)
		})
		.collect();

	for i in 0..cells.len() {
		for j in (i + 1)..cells.len() {
			if !cells_edge_adjacent(&cells[i], &cells[j], EPS) {
				continue;
			}
			let Some((along_x, lo, hi, mid)) =
				shared_edge_span(cells[i].bounds, cells[j].bounds)
			else {
				continue;
			};
			let y0 = Vec3::from(filled[i].1.bounds.min).y;
			let y1 = Vec3::from(filled[i].1.bounds.max).y;
			let height = (y1 - y0).max(2.0);
			let door = connecting_passage(
				along_x,
				lo,
				hi,
				mid,
				y0,
				y1,
				apartment_id,
				cells[i].id,
				cells[j].id,
			);
			if let Some((id, opening)) = door {
				openings.insert(id.clone(), opening.clone());
				let mut wall_openings = Openings::new();
				wall_openings.insert(id, opening);
				if let Some(wall) =
					partition_strip(along_x, lo, hi, mid, y0, height, thickness, &wall_openings)
				{
					partitions.push(wall);
				}
			} else if let Some(wall) =
				partition_strip(along_x, lo, hi, mid, y0, height, thickness, &Openings::new())
			{
				partitions.push(wall);
			}
		}
	}
	(partitions, openings)
}

fn connecting_passage(
	along_x: bool,
	lo: f32,
	hi: f32,
	mid: f32,
	y0: f32,
	y1: f32,
	apartment_id: u32,
	a: u32,
	b: u32,
) -> Option<(OpeningId, Opening)> {
	let shared = hi - lo;
	if shared < DOOR_WIDTH + EPS {
		return None;
	}
	let clear = DOOR_WIDTH.min(shared - 0.15);
	let center = 0.5 * (lo + hi);
	let half = clear * 0.5;
	let door_lo = (center - half).max(lo);
	let door_hi = (center + half).min(hi);
	let half_d = (DEFAULT_PANEL_THICKNESS * 0.5 + 0.06).max(0.12);
	let door_h = (y1 - y0).min(2.2);
	let bounds = if along_x {
		Aabb3d::from_min_max(
			Vec3::new(door_lo, y0, mid - half_d),
			Vec3::new(door_hi, y0 + door_h, mid + half_d),
		)
	} else {
		Aabb3d::from_min_max(
			Vec3::new(mid - half_d, y0, door_lo),
			Vec3::new(mid + half_d, y0 + door_h, door_hi),
		)
	};
	Some((
		OpeningId::scoped(SCOPE, "connect", format!("{apartment_id}_{a}_{b}")),
		Opening::new(bounds, OpeningLabel::Passage),
	))
}

fn shared_edge_span(a: Aabb2d, b: Aabb2d) -> Option<(bool, f32, f32, f32)> {
	let touch_x = (a.max.x - b.min.x).abs() <= EPS || (b.max.x - a.min.x).abs() <= EPS;
	if touch_x {
		let mid = if (a.max.x - b.min.x).abs() <= EPS {
			a.max.x
		} else {
			b.max.x
		};
		let lo = a.min.y.max(b.min.y);
		let hi = a.max.y.min(b.max.y);
		if hi - lo > EPS {
			return Some((false, lo, hi, mid));
		}
	}
	let touch_y = (a.max.y - b.min.y).abs() <= EPS || (b.max.y - a.min.y).abs() <= EPS;
	if touch_y {
		let mid = if (a.max.y - b.min.y).abs() <= EPS {
			a.max.y
		} else {
			b.max.y
		};
		let lo = a.min.x.max(b.min.x);
		let hi = a.max.x.min(b.max.x);
		if hi - lo > EPS {
			return Some((true, lo, hi, mid));
		}
	}
	None
}

fn partition_strip(
	along_x: bool,
	lo: f32,
	hi: f32,
	mid: f32,
	y0: f32,
	height: f32,
	thickness: f32,
	openings: &Openings,
) -> Option<ClippedRectangularStrip> {
	if (hi - lo).abs() < EPS {
		return None;
	}
	let (start, end) = if along_x {
		(Vec3::new(lo, y0, mid), Vec3::new(hi, y0, mid))
	} else {
		(Vec3::new(mid, y0, lo), Vec3::new(mid, y0, hi))
	};
	let outward = if along_x { Vec2::Y } else { Vec2::X };
	let edge = WallEdge::new(start, end, height, outward);
	Some(wall_strip_with_openings(edge, openings, thickness))
}

fn wall_strip_with_openings(
	edge: WallEdge,
	openings: &Openings,
	thickness: f32,
) -> ClippedRectangularStrip {
	let thickness = thickness.max(1e-4);
	let len = edge.length();
	let h = edge.height;
	let tang = edge.tangent();
	let style = PanelStyle::RoughStonework;

	let mut cuts: Vec<(f32, f32, f32, f32)> = Vec::new();
	for (_id, opening) in openings.iter() {
		if !matches!(opening.label, OpeningLabel::Passage) {
			continue;
		}
		let Some(face) = standing_face_opening(edge, &opening.bounds, thickness) else {
			continue;
		};
		let s_lo = face.inset.bottom.clamp(0.0, len);
		let s_hi = (len - face.inset.top).clamp(0.0, len);
		if s_hi - s_lo < EPS {
			continue;
		}
		cuts.push((s_lo, s_hi, face.inset.left, face.inset.right));
	}
	cuts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

	if cuts.is_empty() {
		return ClippedRectangularStrip::from_nodes(
			style,
			[
				RectangularStripNode::new(edge.start, h, thickness, 0.0),
				RectangularStripNode::new(edge.end, h, thickness, 0.0),
			],
			[None],
		);
	}

	let mut nodes = Vec::new();
	let mut insets: Vec<Option<RectInset>> = Vec::new();
	nodes.push(RectangularStripNode::new(edge.start, h, thickness, 0.0));
	let mut cursor = 0.0_f32;
	for (s_lo, s_hi, sill, header) in cuts {
		if s_lo > cursor + EPS {
			nodes.push(RectangularStripNode::new(
				edge.start + tang * s_lo,
				h,
				thickness,
				0.0,
			));
			insets.push(None);
			cursor = s_lo;
		}
		let s_hi = s_hi.max(cursor + EPS);
		nodes.push(RectangularStripNode::new(
			edge.start + tang * s_hi,
			h,
			thickness,
			0.0,
		));
		let jamb = 0.02_f32.min((s_hi - cursor) * 0.1);
		insets.push(Some(RectInset::new(sill, header, jamb, jamb)));
		cursor = s_hi;
	}
	if cursor < len - EPS {
		nodes.push(RectangularStripNode::new(edge.end, h, thickness, 0.0));
		insets.push(None);
	} else if let Some(last) = nodes.last_mut() {
		last.position = edge.end;
	}
	ClippedRectangularStrip::from_nodes(style, nodes, insets)
}

fn try_shell(confines: &Confines) -> Option<RectFloor> {
	let min = Vec3::from(confines.bounds.min);
	let max = Vec3::from(confines.bounds.max);
	let footprint = Vec2::new((max.x - min.x).max(0.0), (max.z - min.z).max(0.0));
	let height = (max.y - min.y).max(0.0);
	if footprint.x < 1.5 || footprint.y < 1.5 || height < 2.0 {
		return None;
	}
	let center_xz = Vec3::new(0.5 * (min.x + max.x), min.y, 0.5 * (min.z + max.z));
	Some(RectFloor::new(RectFloorParams {
		center_xz,
		footprint,
		storey_height: height,
		openings: confines.openings.clone(),
		floor: RectFloorSlab::Solid,
		ceiling: RectFloorSlab::None,
		..RectFloorParams::default()
	}))
}

impl Fit for LivableApartment {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines(0, confines, noise)
	}
}

impl BuildingComponents for LivableApartment {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		if let Some(shell) = &self.shell {
			out.extend(shell.panel_nodes_for_level(level));
		}
		for wall in &self.partitions {
			out.extend(wall.panel_nodes_for_level(level));
		}
		for room in &self.rooms {
			out.extend(room.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		if let Some(shell) = &self.shell {
			out.extend(shell.joint_nodes_for_level(level));
		}
		for wall in &self.partitions {
			out.extend(wall.joint_nodes_for_level(level));
		}
		for room in &self.rooms {
			out.extend(room.joint_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		let name = format!("Livable {}", self.region_id + 1);
		for part in self.cells.iter() {
			let confines = &part.confines;
			let center = Vec3::from(confines.bounds.center());
			let extents =
				Vec3::from(confines.bounds.max - confines.bounds.min).max(Vec3::splat(1e-4));
			out.push_free(LabelNode::rectangle(
				LabelStyle::Blue,
				&name,
				center,
				extents,
				confines.roll,
			));
		}
		for room in &self.rooms {
			out.extend(room.label_nodes_for_level(level));
		}
		out
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		for room in &self.rooms {
			out.extend(room.furniture_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn apt_with_door(extent: Vec3) -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::new(
				Aabb3d::from_min_max(
					Vec3::new(extent.x * 0.35, 0.0, -0.15),
					Vec3::new(extent.x * 0.65, 2.2, 0.15),
				),
				OpeningLabel::Passage,
			),
		);
		Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, extent),
			0.0,
			openings,
		)
	}

	#[test]
	fn layout_has_entryway_and_rooms() {
		let confines = apt_with_door(Vec3::new(10.0, 3.0, 8.0));
		let (apt, _) =
			LivableApartment::from_confines(0, &confines, NoiseParams::default()).unwrap();
		assert!(
			apt.rooms
				.iter()
				.any(|r| matches!(r, ApartmentRoom::Entryway { .. })),
			"expected entryway"
		);
		assert!(
			apt.rooms.iter().any(|r| matches!(
				r,
				ApartmentRoom::Living(_)
					| ApartmentRoom::Kitchen(_)
					| ApartmentRoom::Bedroom(_)
					| ApartmentRoom::Dining(_)
			)),
			"expected at least one common/private quarter"
		);
	}

	#[test]
	fn larger_apt_gets_bedroom() {
		let confines = apt_with_door(Vec3::new(14.0, 3.0, 10.0));
		let (apt, _) =
			LivableApartment::from_confines(0, &confines, NoiseParams { seed: 3, ..Default::default() })
				.unwrap();
		assert!(
			apt.rooms
				.iter()
				.any(|r| matches!(r, ApartmentRoom::Bedroom(_))),
			"expected a bedroom in ~140 m² apt"
		);
	}
}
