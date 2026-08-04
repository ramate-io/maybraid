//! Pack a primary rectangular region into hall-connected livable apartment groups.
//!
//! Pipeline: [`HallsToShafts`] → hall-edge doors → [`group_cells_to_apartments`] →
//! partition walls → [`LivableApartment`] stubs.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::LabelNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{
	aabb_xz_extent, Confines, FillRegion, FillableRegions, Fit, FitError, MultiConfines, SpaceKind,
};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::shells::{RectFloor, RectFloorParams, RectFloorSlab};
use crate::usage_areas::halls_to_shafts::{HallsToShafts, HallsToShaftsOptions};
use crate::usage_areas::livable_apartment::LivableApartment;
use crate::usage_areas::plan_cells::{
	cell_has_hall_frontage, cells_edge_adjacent, group_cells_to_apartments, PlanCell,
};

const EPS: f32 = 1e-3;
const DOOR_WIDTH: f32 = 1.1;
const MIN_ROOM: f32 = 2.0;
const SCOPE: &str = "livable_apartments";

/// Options for [`LivableApartments::from_confines_with`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LivableApartmentsOptions {
	/// Corridor clear width passed to [`HallsToShafts`] (`None` ⇒ sample).
	pub hall_width: Option<f32>,
}

impl Default for LivableApartmentsOptions {
	fn default() -> Self {
		Self { hall_width: None }
	}
}

/// Hall carve + grouped livable apartments inside one primary rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartments {
	pub confines: Confines,
	pub halls: HallsToShafts,
	pub apartments: Vec<LivableApartment>,
	pub walls: Vec<RectFloor>,
	pub hall_width: f32,
}

impl LivableApartments {
	pub fn from_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines_with(confines, noise, LivableApartmentsOptions::default())
	}

	pub fn from_confines_with(
		confines: &Confines,
		noise: NoiseParams,
		options: LivableApartmentsOptions,
	) -> Result<(Self, FillableRegions), FitError> {
		let fp = aabb_xz_extent(&confines.bounds);
		if fp.x + EPS < MIN_ROOM || fp.y + EPS < MIN_ROOM {
			return Err(FitError::TooSmall {
				reason: "livable_apartments_host",
			});
		}

		let (halls, hts_regions) = HallsToShafts::from_confines_with(
			confines,
			noise,
			HallsToShaftsOptions {
				hall_width: options.hall_width,
			},
		)?;
		let hall_width = halls.hall_width;
		let hall_bands = halls.hall_bands.clone();

		let y0 = Vec3::from(confines.bounds.min).y;
		let y1 = Vec3::from(confines.bounds.max).y;
		let roll = confines.roll;

		let mut rooms: Vec<(u32, Aabb2d, Openings)> = Vec::new();
		let mut residual_within = Vec::new();

		for region in hts_regions.within {
			match region.kind {
				SpaceKind::Hallway => {
					residual_within.push(region);
				}
				SpaceKind::InternalSpace => {
					let xz = host_xz(&region.confines.bounds);
					let id = rooms.len() as u32;
					rooms.push((id, xz, region.confines.openings.clone()));
				}
				_ => residual_within.push(region),
			}
		}

		// No rooms → pass host through as a single apartment attempt.
		if rooms.is_empty() {
			match LivableApartment::from_confines(0, confines) {
				Ok((apt, nested)) => {
					residual_within.extend(nested.within);
					return Ok((
						Self {
							confines: confines.clone(),
							halls,
							apartments: vec![apt],
							walls: Vec::new(),
							hall_width,
						},
						FillableRegions {
							within: residual_within,
							atop: Vec::new(),
						},
					));
				}
				Err(FitError::TooSmall { .. }) => {
					residual_within.push(FillRegion::new(
						SpaceKind::InternalSpace,
						confines.clone(),
					));
					return Ok((
						Self {
							confines: confines.clone(),
							halls,
							apartments: Vec::new(),
							walls: Vec::new(),
							hall_width,
						},
						FillableRegions {
							within: residual_within,
							atop: Vec::new(),
						},
					));
				}
				Err(err) => return Err(err),
			}
		}

		let mut door_openings = Openings::new();
		for (id, xz, base) in &mut rooms {
			door_openings.extend(base);
			if let Some(door) = hall_edge_passage(*xz, &hall_bands, *id, y0, y1) {
				door_openings.insert(door.0.clone(), door.1.clone());
				base.insert(door.0, door.1);
			}
		}

		let cells: Vec<PlanCell> = rooms
			.iter()
			.map(|(id, xz, _)| PlanCell::new(*id, *xz))
			.collect();
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let target_area = cfg.sample_range_f32_4d(25.0, 45.0, c.x, c.y, c.z, 120.0);
		let mut groups = group_cells_to_apartments(
			&cells,
			&hall_bands,
			Vec2::splat(MIN_ROOM),
			target_area,
		);
		// No corridor / no hall frontage → one apartment per residual room.
		if groups.is_empty() {
			groups = cells.iter().map(|c| vec![c.id]).collect();
		}

		let cell_by_id: std::collections::HashMap<u32, usize> = rooms
			.iter()
			.enumerate()
			.map(|(i, (id, _, _))| (*id, i))
			.collect();
		let group_of: std::collections::HashMap<u32, usize> = groups
			.iter()
			.enumerate()
			.flat_map(|(gi, g)| g.iter().map(move |&id| (id, gi)))
			.collect();

		let walls = partition_walls(
			&cells,
			&group_of,
			&door_openings,
			y0,
			y1,
			roll,
		);

		let mut apartments = Vec::new();
		let mut next_id = 0u32;
		for group in &groups {
			let mut parts = Vec::new();
			for &cid in group {
				let Some(&ri) = cell_by_id.get(&cid) else {
					continue;
				};
				let (_, xz, openings) = &rooms[ri];
				let bounds = aabb2_to_aabb3(*xz, y0, y1);
				parts.push(FillRegion::new(
					SpaceKind::InternalSpace,
					Confines::new(bounds, roll, openings.clone()),
				));
			}
			if parts.is_empty() {
				continue;
			}
			let multi = MultiConfines::new(parts);
			match LivableApartment::from_multi(next_id, &multi) {
				Ok((apt, nested)) => {
					next_id = next_id.saturating_add(1);
					apartments.push(apt);
					residual_within.extend(nested.within);
				}
				Err(FitError::TooSmall { .. }) => {
					residual_within.extend(multi.parts);
				}
				Err(err) => return Err(err),
			}
		}

		// Rooms that never joined a hall-frontage group stay as residuals.
		for (id, xz, openings) in &rooms {
			if group_of.contains_key(id) {
				continue;
			}
			residual_within.push(FillRegion::new(
				SpaceKind::InternalSpace,
				Confines::new(aabb2_to_aabb3(*xz, y0, y1), roll, openings.clone()),
			));
		}

		Ok((
			Self {
				confines: confines.clone(),
				halls,
				apartments,
				walls,
				hall_width,
			},
			FillableRegions {
				within: residual_within,
				atop: Vec::new(),
			},
		))
	}
}

impl Fit for LivableApartments {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines(confines, noise)
	}
}

impl BuildingComponents for LivableApartments {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		for apt in &self.apartments {
			out.extend(apt.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for wall in &self.walls {
			out.extend(wall.joint_nodes_for_level(level));
		}
		for apt in &self.apartments {
			out.extend(apt.joint_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for apt in &self.apartments {
			out.extend(apt.label_nodes_for_level(level));
		}
		out
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

fn aabb2_to_aabb3(a: Aabb2d, y0: f32, y1: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(a.min.x, y0, a.min.y),
		Vec3::new(a.max.x, y1, a.max.y),
	)
}

fn hall_edge_passage(
	room: Aabb2d,
	halls: &[Aabb2d],
	room_id: u32,
	y0: f32,
	y1: f32,
) -> Option<(OpeningId, Opening)> {
	let cell = PlanCell::new(room_id, room);
	if !cell_has_hall_frontage(&cell, halls, EPS) {
		return None;
	}
	let mut best: Option<(bool, f32, f32, f32, f32)> = None;
	for hall in halls {
		if let Some(span) = shared_edge_span(room, *hall) {
			let len = span.2 - span.1;
			match best {
				None => best = Some((span.0, span.1, span.2, span.3, len)),
				Some((_, _, _, _, bl)) if len > bl => {
					best = Some((span.0, span.1, span.2, span.3, len));
				}
				_ => {}
			}
		}
	}
	let (along_x, lo, hi, mid, shared_len) = best?;
	if shared_len < DOOR_WIDTH + EPS {
		return None;
	}
	let clear = DOOR_WIDTH.min(shared_len - 0.2);
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
		OpeningId::scoped(SCOPE, "hall_door", room_id.to_string()),
		Opening::new(bounds, OpeningLabel::Passage),
	))
}

/// `(along_x, lo_along, hi_along, mid_perp)`.
fn shared_edge_span(a: Aabb2d, b: Aabb2d) -> Option<(bool, f32, f32, f32)> {
	// Touch in X, overlap in Z → vertical joint.
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

fn partition_walls(
	cells: &[PlanCell],
	group_of: &std::collections::HashMap<u32, usize>,
	openings: &Openings,
	y0: f32,
	y1: f32,
	roll: f32,
) -> Vec<RectFloor> {
	let thickness = DEFAULT_PANEL_THICKNESS.max(0.12);
	let height = (y1 - y0).max(2.0);
	let mut walls = Vec::new();
	for i in 0..cells.len() {
		for j in (i + 1)..cells.len() {
			let a = &cells[i];
			let b = &cells[j];
			if !cells_edge_adjacent(a, b, EPS) {
				continue;
			}
			let ga = group_of.get(&a.id).copied();
			let gb = group_of.get(&b.id).copied();
			match (ga, gb) {
				(Some(x), Some(y)) if x != y => {}
				_ => continue,
			}
			let Some((along_x, lo, hi, mid)) = shared_edge_span(a.bounds, b.bounds) else {
				continue;
			};
			let len = (hi - lo).max(thickness);
			let center_along = 0.5 * (lo + hi);
			let (center_xz, footprint) = if along_x {
				(
					Vec3::new(center_along, y0, mid),
					Vec2::new(len, thickness),
				)
			} else {
				(
					Vec3::new(mid, y0, center_along),
					Vec2::new(thickness, len),
				)
			};
			let wall_openings = openings_on_edge(openings, along_x, mid, lo, hi);
			walls.push(RectFloor::new(RectFloorParams {
				center_xz,
				footprint,
				storey_height: height,
				openings: wall_openings,
				floor: RectFloorSlab::None,
				ceiling: RectFloorSlab::None,
				..RectFloorParams::default()
			}));
			let _ = roll;
		}
	}
	walls
}

fn openings_on_edge(
	openings: &Openings,
	along_x: bool,
	mid: f32,
	lo: f32,
	hi: f32,
) -> Openings {
	let mut out = Openings::new();
	for (id, opening) in openings.iter() {
		if !matches!(opening.label, OpeningLabel::Passage) {
			continue;
		}
		let min = Vec3::from(opening.bounds.min);
		let max = Vec3::from(opening.bounds.max);
		let cx = 0.5 * (min.x + max.x);
		let cz = 0.5 * (min.z + max.z);
		let on_edge = if along_x {
			(cz - mid).abs() < 0.35 && cx >= lo - EPS && cx <= hi + EPS
		} else {
			(cx - mid).abs() < 0.35 && cz >= lo - EPS && cz <= hi + EPS
		};
		if on_edge {
			out.insert(id.clone(), opening.clone());
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::OpeningId;

	fn host_with_shafts_and_passage() -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("s0"),
			Opening::new(
				Aabb3d::from_min_max(
					Vec3::new(-6.0, 0.0, -1.0),
					Vec3::new(-4.0, 3.0, 1.0),
				),
				OpeningLabel::Shaft,
			),
		);
		openings.insert(
			OpeningId::new("s1"),
			Opening::new(
				Aabb3d::from_min_max(
					Vec3::new(4.0, 0.0, -1.0),
					Vec3::new(6.0, 3.0, 1.0),
				),
				OpeningLabel::Shaft,
			),
		);
		openings.insert(
			OpeningId::new("p0"),
			Opening::new(
				Aabb3d::from_min_max(
					Vec3::new(-0.6, 0.0, 7.7),
					Vec3::new(0.6, 2.2, 8.1),
				),
				OpeningLabel::Passage,
			),
		);
		Confines::new(
			Aabb3d::from_min_max(
				Vec3::new(-12.0, 0.0, -8.0),
				Vec3::new(12.0, 3.5, 8.0),
			),
			0.0,
			openings,
		)
	}

	#[test]
	fn packs_halls_and_apartments() {
		let confines = host_with_shafts_and_passage();
		let (block, regions) = LivableApartments::from_confines_with(
			&confines,
			NoiseParams::default(),
			LivableApartmentsOptions {
				hall_width: Some(2.5),
			},
		)
		.unwrap();
		assert!(!block.halls.hall_bands.is_empty());
		assert!(!block.apartments.is_empty());
		// At most one hall door per residual room authored into apartment cells.
		let door_count = block
			.apartments
			.iter()
			.flat_map(|a| a.cells.iter())
			.flat_map(|p| p.confines.openings.iter())
			.filter(|(id, _)| id.as_str().contains("hall_door"))
			.count();
		assert!(door_count >= 1, "expected hall doors on rooms");
		let _ = regions;
	}
}
