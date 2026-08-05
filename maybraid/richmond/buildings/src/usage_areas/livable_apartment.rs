//! Livable apartment: entryway → max-rect decomposition → passage tree → RLA.
//!
//! Layout:
//! 1. Carve an **entryway** box at the hall door (open).
//! 2. Map total m² to a room-count program.
//! 3. [`decompose_max_rects`] the remaining footprint.
//! 4. Wire rects with a **spanning tree** of passages rooted at the entry.
//! 5. Fit each rect with [`RectangularLivableArea`] (open/closed normalize).
//! 6. Aggregate rooms / partitions / walkways; leftover scraps → InternalSpace
//!    or household closet. [`crate::IApartmentFullStorey`] maps leftover
//!    InternalSpace → ClosetSpace.

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::{aabb2_area, NoiseConfig, NoiseParams};
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::{LabelNode, LabelStyle};
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{
	Confines, FillRegion, FillableRegions, Fit, FitError, MultiConfines, SpaceKind,
};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::shells::RectFloor;
use crate::usage_areas::common_bedroom::CommonBedroom;
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::livable_quarters::{
	DiningRoom, Kitchen, LivingRoom, ResidentialBathroom, ResidentialHalfBathroom, SittingRoom,
	Study,
};
use crate::usage_areas::plan_cells::{
	decompose_max_rects, shared_edge_span, subtract_aabb2,
};
use crate::usage_areas::rectangular_livable_area::{
	RectAreaRoom, RectLivableStrategy, RectQuarterKind, RectangularLivableArea,
	RectangularLivableAreaParameterized, DEFAULT_MIN_HALL,
};

const EPS: f32 = 1e-3;
const DOOR_WIDTH: f32 = 1.0;
const ENTRY_DEPTH: f32 = 1.8;
const ENTRY_WIDTH: f32 = 2.2;
const MIN_ROOM: f32 = 2.2;
/// Clear width of apartment walkway / access (m).
const WALK_WIDTH: f32 = DEFAULT_MIN_HALL;
/// Minimum shared-edge length (m) for inter-rect graph edges.
const MIN_HALL_ACCESS: f32 = DEFAULT_MIN_HALL;
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
	/// Open hall band from a rectangular livable area (no furniture).
	OpenHall {
		label: LabelNode,
		confines: Confines,
	},
}

impl ApartmentRoom {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Entryway { .. }
			| Self::HouseholdCloset { .. }
			| Self::OpenHall { .. } => Layers::new(),
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
			Self::Entryway { .. }
			| Self::HouseholdCloset { .. }
			| Self::OpenHall { .. } => Layers::new(),
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
			Self::Entryway { label, .. }
			| Self::HouseholdCloset { label, .. }
			| Self::OpenHall { label, .. } => {
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
			Self::Entryway { .. }
			| Self::HouseholdCloset { .. }
			| Self::OpenHall { .. } => Layers::new(),
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

	fn is_closed(&self) -> bool {
		matches!(
			self,
			Self::Bedroom(_) | Self::Bathroom(_) | Self::HalfBath(_) | Self::Study(_)
		)
	}

	fn is_open_circ(&self) -> bool {
		matches!(
			self,
			Self::Entryway { .. }
				| Self::OpenHall { .. }
				| Self::Living(_)
				| Self::Kitchen(_)
				| Self::Dining(_)
				| Self::Sitting(_)
		)
	}
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

/// One apartment group: envelope + entryway / rectangular livable areas.
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartment {
	pub region_id: u32,
	/// Envelope cells that make up this apartment.
	pub cells: MultiConfines,
	/// Packed spaces (entryway, quarters, household closets, open halls).
	pub rooms: Vec<ApartmentRoom>,
	/// Unwalled circulation bands (≥ [`WALK_WIDTH`]); identification only.
	pub walkways: Vec<Aabb2d>,
	/// Partition strips for bedrooms / bathrooms (with connecting passages).
	pub partitions: Vec<ClippedRectangularStrip>,
	/// Max-rect hosts used for layout (gizmos / debug).
	pub max_rects: Vec<Aabb2d>,
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
		let mut walkways = Vec::new();
		let mut partitions = Vec::new();

		// --- Entryway on the door cell ------------------------------------
		let (door_ci, mut work_rects) = collect_work_rects(cells);
		let door_cell = host_xz(&cells.parts[door_ci].confines.bounds);
		let door_opening = find_entry_door(&cells.parts[door_ci].confines.openings)
			.or_else(|| find_entry_door(&cells.parts[0].confines.openings));

		let mut entry_xz = None;
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
					confines: entry_c,
				});
				walkways.push(entry);
				entry_xz = Some(entry);
				// Replace the door-cell footprint with carve remainders (do not
				// leave the uncut cell in `work_rects`).
				work_rects = work_rects
					.into_iter()
					.filter(|r| !aabb2_near_eq(*r, door_cell))
					.chain(rem)
					.filter(|r| aabb2_area(*r) > EPS * EPS)
					.collect();
			}
		}

		work_rects.retain(|r| {
			let s = r.max - r.min;
			s.x + EPS >= MIN_ROOM && s.y + EPS >= MIN_ROOM && aabb2_area(*r) > 4.0
		});
		if work_rects.is_empty() {
			if rooms.is_empty() {
				return Err(FitError::TooSmall {
					reason: "livable_no_body",
				});
			}
			return Ok((
				Self {
					region_id,
					cells: cells.clone(),
					rooms,
					walkways,
					partitions: Vec::new(),
					max_rects: Vec::new(),
					shell: None,
				},
				FillableRegions {
					within: residual_within,
					atop: Vec::new(),
				},
			));
		}

		// --- Max-rect decomposition ---------------------------------------
		let mut max_rects = decompose_max_rects(&work_rects);
		// Drop thin scraps that survived as maximal leaves (e.g. entry flanks).
		max_rects.retain(|r| {
			let s = r.max - r.min;
			s.x + EPS >= MIN_ROOM && s.y + EPS >= MIN_ROOM && aabb2_area(*r) > 8.0
		});
		if max_rects.is_empty() {
			return Err(FitError::TooSmall {
				reason: "livable_no_rects",
			});
		}

		// --- Passage spanning tree ----------------------------------------
		let root = pick_root_rect(&max_rects, entry_xz);
		let tree_edges = spanning_tree_edges(&max_rects, root, MIN_HALL_ACCESS);
		let mut rect_openings: Vec<Openings> = (0..max_rects.len()).map(|_| Openings::new()).collect();

		// Entry → root passage when they share an edge.
		if let Some(entry) = entry_xz {
			if let Some((along_x, lo, hi, mid)) = shared_edge_span(entry, max_rects[root]) {
				if hi - lo + EPS >= MIN_HALL_ACCESS {
					if let Some((id, opening)) = connecting_passage(
						along_x,
						lo,
						hi,
						mid,
						y0,
						y1,
						region_id,
						9990,
						root as u32,
					) {
						rect_openings[root].insert(id, opening);
					}
				}
			}
		}

		for &(a, b) in &tree_edges {
			let Some((along_x, lo, hi, mid)) = shared_edge_span(max_rects[a], max_rects[b]) else {
				continue;
			};
			let Some((id, opening)) = connecting_passage(
				along_x,
				lo,
				hi,
				mid,
				y0,
				y1,
				region_id,
				a as u32,
				b as u32,
			) else {
				continue;
			};
			rect_openings[a].insert(id.clone(), opening.clone());
			rect_openings[b].insert(id, opening);
		}

		// Program slices by area share.
		let kind_list = full_kind_list(program);
		let slices = distribute_program(&kind_list, &max_rects);

		for (ri, rect) in max_rects.iter().enumerate() {
			let confines = confines_from_xz(*rect, y0, y1, roll, &rect_openings[ri]);
			// Seed without ports still needs a synthetic lip toward root/entry
			// so AllOpen normalize can succeed when tree edge insert failed.
			let confines = ensure_passage_or_synthetic(
				confines,
				*rect,
				entry_xz,
				&max_rects,
				root,
				ri,
				y0,
				y1,
				region_id,
			);
			let params = RectangularLivableAreaParameterized {
				strategy: RectLivableStrategy::CaseAttempt,
				min_hall: WALK_WIDTH,
				closed_max_area: 36.0,
			};
			let cell_noise = noise_for_cell(noise, (region_id as i32).wrapping_add(ri as i32 * 17));
			match RectangularLivableArea::fit_with_params(
				&confines,
				cell_noise,
				params,
				&slices[ri],
			) {
				Ok((rla, nested)) => {
					walkways.extend(rla.walkways.iter().copied());
					partitions.extend(rla.partitions);
					for room in rla.rooms {
						push_mapped_rla_room(&mut rooms, room);
					}
					residual_within.extend(nested.within);
				}
				Err(FitError::TooSmall { .. }) => {
					// Soft-fail: whole rect as open hall / residual.
					let c = confines;
					rooms.push(ApartmentRoom::OpenHall {
						label: label_filling_aabb(
							LabelStyle::Cyan,
							"OpenHall",
							&c.bounds,
							roll,
						),
						confines: c.clone(),
					});
					walkways.push(*rect);
					if aabb2_area(*rect) < 8.0 {
						residual_within.push(FillRegion::new(SpaceKind::InternalSpace, c));
					}
				}
				Err(err) => return Err(err),
			}
		}

		// Scraps from original work not covered by max_rects (should be tiny).
		let covered_area: f32 = max_rects.iter().map(|r| aabb2_area(*r)).sum();
		let work_area: f32 = work_rects.iter().map(|r| aabb2_area(*r)).sum();
		if work_area > covered_area + 1.0 {
			for scrap in &work_rects {
				let leftover = subtract_aabb2(*scrap, &max_rects);
				for s in leftover {
					if aabb2_area(s) < EPS {
						continue;
					}
					let c = confines_from_xz(s, y0, y1, roll, &Openings::new());
					push_leftover(&mut rooms, &mut residual_within, c);
				}
			}
		}

		if rooms.is_empty() {
			return Err(FitError::TooSmall {
				reason: "livable_no_quarters",
			});
		}

		// Apartment-level normalize: demote closed rooms that cannot reach entry.
		normalize_apartment_circulation(&mut rooms, entry_xz, MIN_HALL_ACCESS);

		Ok((
			Self {
				region_id,
				cells: cells.clone(),
				rooms,
				walkways,
				partitions,
				max_rects,
				shell: None,
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

fn push_mapped_rla_room(rooms: &mut Vec<ApartmentRoom>, room: RectAreaRoom) {
	match room {
		RectAreaRoom::OpenBand { label, confines } => {
			rooms.push(ApartmentRoom::OpenHall { label, confines });
		}
		RectAreaRoom::HouseholdCloset { label, confines } => {
			rooms.push(ApartmentRoom::HouseholdCloset { label, confines });
		}
		RectAreaRoom::Bedroom(r) => rooms.push(ApartmentRoom::Bedroom(r)),
		RectAreaRoom::Living(r) => rooms.push(ApartmentRoom::Living(r)),
		RectAreaRoom::Eating(e) => {
			rooms.push(ApartmentRoom::Kitchen(e.kitchen));
			if let Some(d) = e.dining {
				rooms.push(ApartmentRoom::Dining(d));
			}
		}
		RectAreaRoom::Kitchen(r) => rooms.push(ApartmentRoom::Kitchen(r)),
		RectAreaRoom::Dining(r) => rooms.push(ApartmentRoom::Dining(r)),
		RectAreaRoom::Bathroom(r) => rooms.push(ApartmentRoom::Bathroom(r)),
		RectAreaRoom::HalfBath(r) => rooms.push(ApartmentRoom::HalfBath(r)),
		RectAreaRoom::Sitting(r) => rooms.push(ApartmentRoom::Sitting(r)),
		RectAreaRoom::Study(r) => rooms.push(ApartmentRoom::Study(r)),
	}
}

fn program_from_area(area: f32, noise: NoiseParams, center: Vec3) -> ProgramCounts {
	let cfg = NoiseConfig::new(noise);
	let jitter = cfg.sample_range_f32_4d(0.0, 1.0, center.x, center.y, center.z, 44.0);
	// Prefer an eating area in nearly every apartment; only skip on tiny footprints.
	let want_kitchen = area >= 22.0 || jitter > 0.2;
	if area < 36.0 {
		ProgramCounts {
			bedrooms: 0,
			bathrooms: 1,
			half_baths: 0,
			kitchens: if want_kitchen { 1 } else { 0 },
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
			dining: if jitter > 0.35 { 1 } else { 0 },
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

fn full_kind_list(p: ProgramCounts) -> Vec<RectQuarterKind> {
	let mut out = Vec::new();
	// Eating before living so kitchens win the first large open claim.
	if p.kitchens > 0 || p.dining > 0 {
		out.push(RectQuarterKind::Eating);
	}
	for _ in 0..p.living {
		out.push(RectQuarterKind::Living);
	}
	for _ in 0..p.sitting {
		out.push(RectQuarterKind::Sitting);
	}
	for _ in 0..p.bedrooms {
		out.push(RectQuarterKind::Bedroom);
	}
	for _ in 0..p.bathrooms {
		out.push(RectQuarterKind::Bathroom);
	}
	for _ in 0..p.half_baths {
		out.push(RectQuarterKind::HalfBath);
	}
	for _ in 0..p.studies {
		out.push(RectQuarterKind::Study);
	}
	if out.is_empty() {
		out.push(RectQuarterKind::Living);
	}
	out
}

fn distribute_program(kinds: &[RectQuarterKind], rects: &[Aabb2d]) -> Vec<Vec<RectQuarterKind>> {
	let n = rects.len();
	let mut slices = vec![Vec::new(); n];
	if n == 0 {
		return slices;
	}
	let areas: Vec<f32> = rects.iter().map(|r| aabb2_area(*r)).collect();
	let total: f32 = areas.iter().sum::<f32>().max(EPS);
	// Closed first, then open. Living/sitting prefer larger rects; eating takes a
	// mid-size claim so it is satisfied without monopolizing the biggest pocket.
	let mut ordered: Vec<RectQuarterKind> = kinds.to_vec();
	ordered.sort_by_key(|k| match k {
		k if k.is_closed() => 0u8,
		RectQuarterKind::Living | RectQuarterKind::Sitting => 1u8,
		RectQuarterKind::Eating => 2u8,
		_ => 3u8,
	});
	let mut load = vec![0.0_f32; n];
	let targets: Vec<f32> = areas.iter().map(|a| a / total).collect();
	for kind in ordered {
		let mut best = 0usize;
		let mut best_score = f32::NEG_INFINITY;
		for i in 0..n {
			let closed_bonus = if kind.is_closed() { areas[i] * 0.02 } else { 0.0 };
			let living_bonus =
				if matches!(kind, RectQuarterKind::Living | RectQuarterKind::Sitting) {
					areas[i] * 0.04
				} else {
					0.0
				};
			// Mild penalty on the largest rects so eating leaves them for living.
			let eating_penalty = if matches!(kind, RectQuarterKind::Eating) {
				areas[i] * 0.015
			} else {
				0.0
			};
			let score = targets[i] - load[i] / total.max(1.0) + areas[i] * 1e-4 + closed_bonus
				+ living_bonus
				- eating_penalty;
			if score > best_score {
				best_score = score;
				best = i;
			}
		}
		slices[best].push(kind);
		load[best] += if kind.is_closed() {
			1.5
		} else if matches!(kind, RectQuarterKind::Living | RectQuarterKind::Sitting) {
			1.35
		} else {
			1.0
		};
	}
	for s in &mut slices {
		if s.is_empty() {
			s.push(RectQuarterKind::Living);
		}
		// Within a rect: eat first (compact carve), then living/sitting take the rest.
		s.sort_by_key(|k| match k {
			k if k.is_closed() => 0u8,
			RectQuarterKind::Eating | RectQuarterKind::Kitchen => 1u8,
			RectQuarterKind::Living | RectQuarterKind::Sitting => 2u8,
			_ => 3u8,
		});
	}
	slices
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
	let width = width
		.clamp(1.4, host.max.x - host.min.x - EPS)
		.min(host.max.y - host.min.y - EPS);

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
	let rem = subtract_aabb2(host, &[entry]);
	Some((entry, rem))
}

fn pick_root_rect(rects: &[Aabb2d], entry: Option<Aabb2d>) -> usize {
	if let Some(entry) = entry {
		let mut best = 0usize;
		let mut best_score = f32::NEG_INFINITY;
		for (i, r) in rects.iter().enumerate() {
			let score = shared_edge_span(entry, *r)
				.map(|(_, lo, hi, _)| hi - lo)
				.unwrap_or(0.0)
				+ 1.0 / (1.0 + (entry.center() - r.center()).length());
			if score > best_score {
				best_score = score;
				best = i;
			}
		}
		return best;
	}
	0
}

fn spanning_tree_edges(rects: &[Aabb2d], root: usize, min_access: f32) -> Vec<(usize, usize)> {
	let n = rects.len();
	if n <= 1 {
		return Vec::new();
	}
	let mut adj: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
	for i in 0..n {
		for j in (i + 1)..n {
			if let Some((_, lo, hi, _)) = shared_edge_span(rects[i], rects[j]) {
				let len = hi - lo;
				if len + EPS >= min_access {
					adj[i].push((j, len));
					adj[j].push((i, len));
				}
			}
		}
	}
	// Prim from root, prefer longer shared edges.
	let mut in_tree = vec![false; n];
	in_tree[root] = true;
	let mut edges = Vec::new();
	for _ in 1..n {
		let mut best: Option<(usize, usize, f32)> = None;
		for i in 0..n {
			if !in_tree[i] {
				continue;
			}
			for &(j, len) in &adj[i] {
				if in_tree[j] {
					continue;
				}
				if best.map(|(_, _, l)| len > l).unwrap_or(true) {
					best = Some((i, j, len));
				}
			}
		}
		let Some((a, b, _)) = best else {
			// Disconnected: attach nearest by center distance with a synthetic link skip.
			break;
		};
		in_tree[b] = true;
		edges.push((a, b));
	}
	edges
}

fn ensure_passage_or_synthetic(
	mut confines: Confines,
	rect: Aabb2d,
	entry: Option<Aabb2d>,
	rects: &[Aabb2d],
	root: usize,
	ri: usize,
	y0: f32,
	y1: f32,
	region_id: u32,
) -> Confines {
	let has = confines
		.openings
		.iter()
		.any(|(_, o)| matches!(o.label, OpeningLabel::Passage));
	if has {
		return confines;
	}
	// Prefer contact with entry, else with root rect.
	let target = entry.or_else(|| rects.get(root).copied());
	if let Some(t) = target {
		if let Some((along_x, lo, hi, mid)) = shared_edge_span(rect, t) {
			if let Some((id, opening)) =
				connecting_passage(along_x, lo, hi, mid, y0, y1, region_id, ri as u32, 8888)
			{
				confines.openings.insert(id, opening);
				return confines;
			}
		}
	}
	// South-face synthetic door so RLA strategies have a port.
	let door_w = DOOR_WIDTH.min(rect.max.x - rect.min.x - 0.2).max(0.7);
	let half = door_w * 0.5;
	let cx = 0.5 * (rect.min.x + rect.max.x);
	let door_h = (y1 - y0).min(2.15).max(1.9);
	confines.openings.insert(
		OpeningId::scoped(SCOPE, "synthetic", format!("{region_id}_{ri}")),
		Opening::new(
			Aabb3d::from_min_max(
				Vec3::new(cx - half, y0, rect.min.y - 0.12),
				Vec3::new(cx + half, y0 + door_h, rect.min.y + 0.12),
			),
			OpeningLabel::Passage,
		),
	);
	confines
}

fn room_xz(room: &ApartmentRoom) -> Option<Aabb2d> {
	match room {
		ApartmentRoom::Entryway { confines, .. }
		| ApartmentRoom::HouseholdCloset { confines, .. }
		| ApartmentRoom::OpenHall { confines, .. } => Some(host_xz(&confines.bounds)),
		ApartmentRoom::Bedroom(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Living(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Kitchen(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Dining(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Bathroom(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::HalfBath(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Sitting(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Study(r) => Some(label_xz(&r.room_type)),
	}
}

fn label_xz(label: &LabelNode) -> Aabb2d {
	let c = label.placement.translation;
	let e = label.placement.scale;
	Aabb2d {
		min: Vec2::new(c.x - e.x * 0.5, c.z - e.z * 0.5),
		max: Vec2::new(c.x + e.x * 0.5, c.z + e.z * 0.5),
	}
}

/// Soft normalize: closed rooms must path-connect to entry via open regions.
/// Violators become household closets / are left as-is only when already soft.
fn normalize_apartment_circulation(
	rooms: &mut Vec<ApartmentRoom>,
	entry: Option<Aabb2d>,
	min_hall: f32,
) {
	let Some(entry) = entry else {
		return;
	};
	let open_rects: Vec<Aabb2d> = rooms
		.iter()
		.filter(|r| r.is_open_circ())
		.filter_map(room_xz)
		.chain(std::iter::once(entry))
		.collect();
	if open_rects.is_empty() {
		return;
	}
	// BFS from entry through open rects.
	let mut reach = vec![false; open_rects.len()];
	let mut stack = Vec::new();
	for (i, r) in open_rects.iter().enumerate() {
		if aabb2_near_eq(*r, entry)
			|| shared_edge_span(*r, entry).is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_hall * 0.5)
		{
			reach[i] = true;
			stack.push(i);
		}
	}
	if stack.is_empty() {
		reach[0] = true;
		stack.push(0);
	}
	while let Some(i) = stack.pop() {
		for j in 0..open_rects.len() {
			if reach[j] {
				continue;
			}
			let touch = shared_edge_span(open_rects[i], open_rects[j])
				.is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_hall * 0.4);
			if touch {
				reach[j] = true;
				stack.push(j);
			}
		}
	}
	let reachable_open: Vec<Aabb2d> = open_rects
		.iter()
		.zip(reach.iter())
		.filter(|(_, r)| **r)
		.map(|(a, _)| *a)
		.collect();

	for room in rooms.iter_mut() {
		if !room.is_closed() {
			continue;
		}
		let Some(cz) = room_xz(room) else {
			continue;
		};
		let ok = reachable_open.iter().any(|o| {
			shared_edge_span(cz, *o).is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_hall * 0.5)
		});
		if !ok {
			// Demote: replace with HouseholdCloset using label bounds.
			let confines = Confines::new(
				Aabb3d::from_min_max(
					Vec3::new(cz.min.x, 0.0, cz.min.y),
					Vec3::new(cz.max.x, 3.0, cz.max.y),
				),
				0.0,
				Openings::new(),
			);
			*room = ApartmentRoom::HouseholdCloset {
				label: label_filling_aabb(
					LabelStyle::Gray,
					"HouseholdCloset",
					&confines.bounds,
					0.0,
				),
				confines,
			};
		}
	}
}

fn push_leftover(rooms: &mut Vec<ApartmentRoom>, residual: &mut Vec<FillRegion>, confines: Confines) {
	let area = {
		let fp = confines.footprint();
		fp.x * fp.y
	};
	if (1.8..8.0).contains(&area) {
		rooms.push(ApartmentRoom::HouseholdCloset {
			label: label_filling_aabb(
				LabelStyle::Gray,
				"HouseholdCloset",
				&confines.bounds,
				confines.roll,
			),
			confines,
		});
	} else {
		residual.push(FillRegion::new(SpaceKind::InternalSpace, confines));
	}
}

fn noise_for_cell(noise: NoiseParams, cell: i32) -> NoiseParams {
	NoiseParams {
		seed: noise.seed.wrapping_add(cell.wrapping_mul(97)),
		..noise
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
	if shared < DOOR_WIDTH * 0.7 + EPS {
		return None;
	}
	let clear = DOOR_WIDTH.min(shared - 0.1).max(0.7);
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
					| ApartmentRoom::OpenHall { .. }
			)),
			"expected at least one common/private/open quarter"
		);
		assert!(!apt.max_rects.is_empty(), "expected max-rect decomposition");
	}

	#[test]
	fn larger_apt_gets_bedroom() {
		let confines = apt_with_door(Vec3::new(14.0, 3.0, 10.0));
		let (apt, _) = LivableApartment::from_confines(
			0,
			&confines,
			NoiseParams {
				seed: 3,
				..Default::default()
			},
		)
		.unwrap();
		assert!(
			apt.rooms
				.iter()
				.any(|r| matches!(r, ApartmentRoom::Bedroom(_))),
			"expected bedroom in larger apt"
		);
	}

	#[test]
	fn l_shape_reaches_entry_from_far() {
		// L: 10×6 bar + 4×6 stub on the west, door on south of bar.
		let bar = FillRegion::new(
			SpaceKind::InternalSpace,
			Confines::new(
				Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 3.0, 6.0)),
				0.0,
				{
					let mut o = Openings::new();
					o.insert(
						OpeningId::new("door"),
						Opening::new(
							Aabb3d::from_min_max(
								Vec3::new(4.0, 0.0, -0.15),
								Vec3::new(5.0, 2.2, 0.15),
							),
							OpeningLabel::Passage,
						),
					);
					o
				},
			),
		);
		let stub = FillRegion::new(
			SpaceKind::InternalSpace,
			Confines::new(
				Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 6.0), Vec3::new(4.0, 3.0, 12.0)),
				0.0,
				Openings::new(),
			),
		);
		let cells = MultiConfines::new([bar, stub]);
		let (apt, _) =
			LivableApartment::from_multi(0, &cells, NoiseParams::default()).unwrap();
		assert!(apt.max_rects.len() >= 2, "L should yield ≥2 max-rects");
		assert!(
			apt.rooms
				.iter()
				.any(|r| matches!(r, ApartmentRoom::Entryway { .. }))
		);
		// Far private/open content should exist in the stub half (z > 6).
		let far = apt.rooms.iter().filter_map(room_xz).any(|r| r.min.y > 5.5);
		assert!(far, "expected packed content in far L leg");
	}
}
