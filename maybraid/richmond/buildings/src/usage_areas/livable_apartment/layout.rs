//! Apartment fill: entry → max-rect cluster → RLA per rect → normalize.

use bevy_math::bounding::Aabb2d;
use bevy_math::{Vec2, Vec3};
use procedural_common::{aabb2_area, inflate_aabb2, NoiseParams};
use richmond_building_components::labels::{LabelNode, LabelStyle};

use crate::fit::{
	Confines, FillRegion, FillableRegions, FitError, MultiConfines, SpaceKind,
};
use crate::openings::Openings;
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::plan_access::PlanAccessParams;
use crate::usage_areas::plan_cells::{shared_edge_span, subtract_aabb2};
use crate::usage_areas::plan_geom::{confines_from_xz, host_xz, noise_for_cell};
use crate::usage_areas::rect_passage_cluster::{RectPassageCluster, RectPassageClusterParams};
use crate::usage_areas::rectangular_livable_area::{
	RectAreaRoom, RectLivableStrategy, RectangularLivableArea, RectangularLivableAreaParameterized,
	DEFAULT_CLOSED_MAX_AREA,
};

use super::entry::{
	collect_work_rects, find_entry_door, partition_entry_and_body, push_entryway,
};
use super::program::{distribute_program, full_kind_list, program_from_area};
use super::room::ApartmentRoom;
use super::{LivableApartment, EPS, SCOPE};

pub(crate) fn residential_access() -> PlanAccessParams {
	PlanAccessParams::residential()
}

pub(crate) fn fit_from_multi(
	region_id: u32,
	cells: &MultiConfines,
	noise: NoiseParams,
) -> Result<(LivableApartment, FillableRegions), FitError> {
	if cells.is_empty() {
		return Err(FitError::TooSmall {
			reason: "livable_empty",
		});
	}
	let access = residential_access();
	let mut has_body = false;
	for part in cells.iter() {
		let height = (part.confines.bounds.max.y - part.confines.bounds.min.y).max(0.0);
		let xz = host_xz(&part.confines.bounds);
		if !access.is_walkable(xz) {
			return Err(FitError::TooSmall {
				reason: "livable_footprint",
			});
		}
		if access.is_room_rect(xz) {
			has_body = true;
		}
		if height < 2.0 {
			return Err(FitError::TooSmall {
				reason: "livable_height",
			});
		}
	}
	if !has_body {
		return Err(FitError::TooSmall {
			reason: "livable_no_body_cell",
		});
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
	let apt_noise = noise_for_cell(noise, region_id as i32);
	let program = program_from_area(
		total_area,
		apt_noise,
		cells.parts[0].confines.center(),
	);

	let mut rooms = Vec::new();
	let mut residual_within = Vec::new();
	let mut walkways = Vec::new();
	let mut partitions = Vec::new();

	let (door_ci, work_rects) = collect_work_rects(cells);
	let door_cell = host_xz(&cells.parts[door_ci].confines.bounds);
	let door_opening = find_entry_door(&cells.parts[door_ci].confines.openings)
		.or_else(|| find_entry_door(&cells.parts[0].confines.openings));

	let partitioned = partition_entry_and_body(
		work_rects,
		door_cell,
		door_opening.as_ref().map(|(_, d)| d),
		access,
	);
	for band in &partitioned.entry_bands {
		push_entryway(&mut rooms, &mut walkways, *band, y0, y1, roll);
	}
	for scrap in partitioned.scraps {
		let c = confines_from_xz(scrap, y0, y1, roll, &Openings::new());
		push_leftover(&mut rooms, &mut residual_within, c);
	}
	let entry_xz = partitioned.entry_bands.first().copied();
	let work_rects = partitioned.body;
	if work_rects.is_empty() {
		if rooms.is_empty() {
			return Err(FitError::TooSmall {
				reason: "livable_no_body",
			});
		}
		return Ok((
			LivableApartment {
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

	let cluster = RectPassageCluster::from_parts(
		&work_rects,
		entry_xz,
		y0,
		y1,
		roll,
		region_id,
		RectPassageClusterParams {
			min_room: access.room_min,
			min_rect_area: 8.0,
			min_access: access.walk_clear,
			scope: SCOPE,
		},
	)
	.ok_or(FitError::TooSmall {
		reason: "livable_no_rects",
	})?;

	let kind_list = full_kind_list(program);
	let slices = distribute_program(&kind_list, &cluster.rects, apt_noise);
	let rla_params = RectangularLivableAreaParameterized {
		strategy: RectLivableStrategy::CaseAttempt,
		min_hall: access.walk_clear,
		closed_max_area: DEFAULT_CLOSED_MAX_AREA,
	};

	for (ri, rect) in cluster.rects.iter().enumerate() {
		let confines = cluster.confines_ensured(ri, entry_xz);
		let cell_noise =
			noise_for_cell(apt_noise, (region_id as i32).wrapping_add(ri as i32 * 17));
		match RectangularLivableArea::fit_with_params(
			&confines,
			cell_noise,
			rla_params,
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
				rooms.push(ApartmentRoom::OpenHall {
					label: label_filling_aabb(
						LabelStyle::Cyan,
						"OpenHall",
						&confines.bounds,
						roll,
					),
					confines: confines.clone(),
				});
				walkways.push(*rect);
				if aabb2_area(*rect) < 8.0 {
					residual_within.push(FillRegion::new(SpaceKind::InternalSpace, confines));
				}
			}
			Err(err) => return Err(err),
		}
	}

	let covered_area: f32 = cluster.rects.iter().map(|r| aabb2_area(*r)).sum();
	let work_area: f32 = work_rects.iter().map(|r| aabb2_area(*r)).sum();
	if work_area > covered_area + 1.0 {
		for scrap in &work_rects {
			let leftover = subtract_aabb2(*scrap, &cluster.rects);
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

	normalize_apartment_circulation(&mut rooms, entry_xz, access, &walkways, y0, y1, roll);

	Ok((
		LivableApartment {
			region_id,
			cells: cells.clone(),
			rooms,
			walkways,
			partitions,
			max_rects: cluster.rects,
			shell: None,
		},
		FillableRegions {
			within: residual_within,
			atop: Vec::new(),
		},
	))
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

pub(crate) fn room_xz(room: &ApartmentRoom) -> Option<Aabb2d> {
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
///
/// Unreachable closed rooms become household closets only when closet-sized;
/// larger demotions reopen as [`ApartmentRoom::OpenHall`] so normalize cannot
/// flood an apartment with oversized "closets".
///
/// Closed rooms from SpineHall / similar strategies sit behind partition
/// thickness, so adjacency uses a wall-gap inflate — exact edge touch would
/// false-demote every walled bedroom into an empty OpenHall while leaving
/// the enclosure panels in place.
fn normalize_apartment_circulation(
	rooms: &mut Vec<ApartmentRoom>,
	entry: Option<Aabb2d>,
	access: PlanAccessParams,
	walkways: &[Aabb2d],
	y0: f32,
	y1: f32,
	roll: f32,
) {
	let Some(entry) = entry else {
		return;
	};
	let door = access.door_contact();
	// Any open circ / walkway counts — not only an entry-flood component.
	// SpineHall bedrooms sit on a spine that may not share a long edge with the
	// entry band (door connectivity is via RLA passages), and requiring
	// entry-reachability false-demotes those walled rooms into OpenHalls.
	let open_rects: Vec<Aabb2d> = rooms
		.iter()
		.filter(|r| r.is_open_circ())
		.filter_map(room_xz)
		.chain(std::iter::once(entry))
		.chain(walkways.iter().copied())
		.collect();
	if open_rects.is_empty() {
		return;
	}

	for room in rooms.iter_mut() {
		if !room.is_closed() {
			continue;
		}
		let Some(cz) = room_xz(room) else {
			continue;
		};
		let ok = open_rects
			.iter()
			.any(|o| closed_reaches_open(cz, *o, door));
		if ok {
			continue;
		}
		let area = aabb2_area(cz);
		let confines = confines_from_xz(cz, y0, y1, roll, &Openings::new());
		// Closet band matches [`push_leftover`]; larger pockets reopen as halls.
		if (1.8..8.0).contains(&area) {
			*room = ApartmentRoom::HouseholdCloset {
				label: label_filling_aabb(
					LabelStyle::Gray,
					"HouseholdCloset",
					&confines.bounds,
					roll,
				),
				confines,
			};
		} else {
			*room = ApartmentRoom::OpenHall {
				label: label_filling_aabb(
					LabelStyle::Cyan,
					"OpenHall",
					&confines.bounds,
					roll,
				),
				confines,
			};
		}
	}
}

/// True when a closed room abuts open circulation, allowing a partition gap.
fn closed_reaches_open(closed: Aabb2d, open: Aabb2d, min_len: f32) -> bool {
	if shared_edge_span(closed, open).is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_len) {
		return true;
	}
	// SpineHall enclosures sit ~panel thickness off the hall band.
	let gap = DEFAULT_PANEL_THICKNESS + 0.2;
	let grown = inflate_aabb2(closed, gap);
	shared_edge_span(grown, open).is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_len)
		|| shared_edge_span(closed, inflate_aabb2(open, gap))
			.is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_len)
}

fn push_leftover(
	rooms: &mut Vec<ApartmentRoom>,
	residual: &mut Vec<FillRegion>,
	confines: Confines,
) {
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
