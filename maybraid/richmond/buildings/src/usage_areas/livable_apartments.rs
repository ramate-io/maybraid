//! Pack a primary rectangular region into hall-connected livable apartment groups.
//!
//! Pipeline: [`HallsToShafts`] → split residuals → [`pack_apartments_to_targets`]
//! → one hall door per group → partition / hall-edge walls → [`LivableApartment`]
//! stubs (no per-cell shells — avoids double-walling).

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
	cell_has_hall_frontage, cells_edge_adjacent, pack_apartments_to_targets, split_oversized_cells,
	split_toward_min_room, PlanCell,
};

const EPS: f32 = 1e-3;
const DOOR_WIDTH: f32 = 1.1;
const MIN_ROOM: f32 = 2.5;
const SCOPE: &str = "livable_apartments";

/// Noise knobs for [`LivableApartments`] (target-area catalog + hall width).
///
/// Analogous to Les Halles stall-door catalogs: sample a list of target square
/// meters, then pack apartment groups against that catalog in order.
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartmentsParameterized {
	/// Corridor clear width for [`HallsToShafts`] (`None` ⇒ sample inside HTS).
	pub hall_width: Option<f32>,
	/// Target apartment areas in m² (catalog order; large → small preferred).
	pub targets: Vec<f32>,
}

impl LivableApartmentsParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let fp = aabb_xz_extent(&confines.bounds);
		if fp.x + EPS < MIN_ROOM || fp.y + EPS < MIN_ROOM {
			return Err(FitError::TooSmall {
				reason: "livable_apartments_host",
			});
		}
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		Ok(Self {
			hall_width: None,
			targets: generate_apartment_targets(&cfg, c),
		})
	}

	pub fn with_hall_width(mut self, hall_width: Option<f32>) -> Self {
		self.hall_width = hall_width;
		self
	}
}

/// Options for [`LivableApartments::from_confines_with`].
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartmentsOptions {
	/// Corridor clear width passed to [`HallsToShafts`] (`None` ⇒ sample).
	pub hall_width: Option<f32>,
	/// Optional pre-authored target areas (m²). Empty / `None` ⇒ sample catalog.
	pub targets: Option<Vec<f32>>,
}

impl Default for LivableApartmentsOptions {
	fn default() -> Self {
		Self {
			hall_width: None,
			targets: None,
		}
	}
}

/// Hall carve + grouped livable apartments inside one primary rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartments {
	pub confines: Confines,
	pub parameterized: LivableApartmentsParameterized,
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
		let mut params = LivableApartmentsParameterized::sample(confines, noise)?;
		params.hall_width = options.hall_width.or(params.hall_width);
		if let Some(targets) = options.targets {
			if !targets.is_empty() {
				params.targets = targets;
			}
		}
		Self::from_parameterized(params, confines, noise)
	}

	pub fn from_parameterized(
		params: LivableApartmentsParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let fp = aabb_xz_extent(&confines.bounds);
		if fp.x + EPS < MIN_ROOM || fp.y + EPS < MIN_ROOM {
			return Err(FitError::TooSmall {
				reason: "livable_apartments_host",
			});
		}
		if params.targets.is_empty() {
			return Err(FitError::InvalidConfines {
				reason: "livable_apartments_empty_targets",
			});
		}

		let (halls, hts_regions) = HallsToShafts::from_confines_with(
			confines,
			noise,
			HallsToShaftsOptions {
				hall_width: params.hall_width,
			},
		)?;
		let hall_width = halls.hall_width;
		let hall_bands = halls.hall_bands.clone();

		let y0 = Vec3::from(confines.bounds.min).y;
		let y1 = Vec3::from(confines.bounds.max).y;
		let roll = confines.roll;

		let mut residual_within = Vec::new();
		let mut seed_cells = Vec::new();
		let mut next_id = 0u32;

		for region in hts_regions.within {
			match region.kind {
				SpaceKind::Hallway => {
					residual_within.push(region);
				}
				SpaceKind::InternalSpace => {
					let xz = host_xz(&region.confines.bounds);
					seed_cells.push(PlanCell::new(next_id, xz));
					next_id = next_id.saturating_add(1);
				}
				_ => residual_within.push(region),
			}
		}

		// No rooms → pass host through as a single apartment attempt (no shell).
		if seed_cells.is_empty() {
			return singleton_host(
				confines,
				params,
				halls,
				hall_width,
				residual_within,
			);
		}

		let min_room = Vec2::splat(MIN_ROOM);
		let mut cells = split_toward_min_room(&seed_cells, min_room, &mut next_id);
		let min_target = params
			.targets
			.iter()
			.copied()
			.fold(f32::INFINITY, f32::min)
			.max(12.0);
		// Dice residuals so groups can form L / multi-rect shapes toward catalog sizes.
		let max_cell_area = (min_target * 0.55).max(min_room.x * min_room.y * 2.0);
		cells = split_oversized_cells(&cells, max_cell_area, min_room, &mut next_id);

		let mut groups = pack_apartments_to_targets(&cells, &hall_bands, min_room, &params.targets);
		if groups.is_empty() {
			groups = cells.iter().map(|c| vec![c.id]).collect();
		}

		let cell_by_id: std::collections::HashMap<u32, usize> = cells
			.iter()
			.enumerate()
			.map(|(i, c)| (c.id, i))
			.collect();
		let group_of: std::collections::HashMap<u32, usize> = groups
			.iter()
			.enumerate()
			.flat_map(|(gi, g)| g.iter().map(move |&id| (id, gi)))
			.collect();

		// One hall door per apartment group (not per residual room).
		let mut door_openings = Openings::new();
		let mut cell_openings: std::collections::HashMap<u32, Openings> = std::collections::HashMap::new();
		for (gi, group) in groups.iter().enumerate() {
			let Some(door) = group_hall_door(group, &cells, &hall_bands, gi as u32, y0, y1) else {
				continue;
			};
			door_openings.insert(door.0.clone(), door.1.clone());
			cell_openings
				.entry(door.2)
				.or_insert_with(Openings::new)
				.insert(door.0, door.1);
		}

		let walls = enclosure_walls(
			&cells,
			&hall_bands,
			&group_of,
			&door_openings,
			y0,
			y1,
		);

		let mut apartments = Vec::new();
		let mut apt_id = 0u32;
		for group in &groups {
			let mut parts = Vec::new();
			for &cid in group {
				let Some(&ci) = cell_by_id.get(&cid) else {
					continue;
				};
				let openings = cell_openings.get(&cid).cloned().unwrap_or_default();
				let bounds = aabb2_to_aabb3(cells[ci].bounds, y0, y1);
				parts.push(FillRegion::new(
					SpaceKind::InternalSpace,
					Confines::new(bounds, roll, openings),
				));
			}
			if parts.is_empty() {
				continue;
			}
			let multi = MultiConfines::new(parts);
			match LivableApartment::from_multi(apt_id, &multi) {
				Ok((mut apt, nested)) => {
					// Partition / hall walls are authored once here — no per-cell shells.
					apt.shell = None;
					apt_id = apt_id.saturating_add(1);
					apartments.push(apt);
					residual_within.extend(nested.within);
				}
				Err(FitError::TooSmall { .. }) => {
					residual_within.extend(multi.parts);
				}
				Err(err) => return Err(err),
			}
		}

		for cell in &cells {
			if group_of.contains_key(&cell.id) {
				continue;
			}
			let openings = cell_openings.get(&cell.id).cloned().unwrap_or_default();
			residual_within.push(FillRegion::new(
				SpaceKind::InternalSpace,
				Confines::new(aabb2_to_aabb3(cell.bounds, y0, y1), roll, openings),
			));
		}

		Ok((
			Self {
				confines: confines.clone(),
				parameterized: params,
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

/// Noise-perturbed apartment target-area catalog (m²), large → small.
fn generate_apartment_targets(cfg: &NoiseConfig, center: Vec3) -> Vec<f32> {
	const BASES: &[f32] = &[55.0, 48.0, 40.0, 35.0, 30.0, 26.0, 22.0, 18.0];
	BASES
		.iter()
		.enumerate()
		.map(|(i, &base)| {
			cfg.sample_range_f32_4d(
				(base - 4.0).max(14.0),
				base + 5.0,
				center.x,
				center.y,
				center.z,
				80.0 + i as f32,
			)
		})
		.collect()
}

fn singleton_host(
	confines: &Confines,
	params: LivableApartmentsParameterized,
	halls: HallsToShafts,
	hall_width: f32,
	mut residual_within: Vec<FillRegion>,
) -> Result<(LivableApartments, FillableRegions), FitError> {
	match LivableApartment::from_confines(0, confines) {
		Ok((mut apt, nested)) => {
			apt.shell = None;
			residual_within.extend(nested.within);
			Ok((
				LivableApartments {
					confines: confines.clone(),
					parameterized: params,
					halls,
					apartments: vec![apt],
					walls: Vec::new(),
					hall_width,
				},
				FillableRegions {
					within: residual_within,
					atop: Vec::new(),
				},
			))
		}
		Err(FitError::TooSmall { .. }) => {
			residual_within.push(FillRegion::new(
				SpaceKind::InternalSpace,
				confines.clone(),
			));
			Ok((
				LivableApartments {
					confines: confines.clone(),
					parameterized: params,
					halls,
					apartments: Vec::new(),
					walls: Vec::new(),
					hall_width,
				},
				FillableRegions {
					within: residual_within,
					atop: Vec::new(),
				},
			))
		}
		Err(err) => Err(err),
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

/// One hall door for the whole group, authored on the best frontage cell.
/// Returns `(id, opening, cell_id)`.
fn group_hall_door(
	group: &[u32],
	cells: &[PlanCell],
	halls: &[Aabb2d],
	group_id: u32,
	y0: f32,
	y1: f32,
) -> Option<(OpeningId, Opening, u32)> {
	let mut best: Option<(u32, bool, f32, f32, f32, f32)> = None;
	for &cid in group {
		let Some(cell) = cells.iter().find(|c| c.id == cid) else {
			continue;
		};
		if !cell_has_hall_frontage(cell, halls, EPS) {
			continue;
		}
		for hall in halls {
			if let Some(span) = shared_edge_span(cell.bounds, *hall) {
				let len = span.2 - span.1;
				match best {
					None => best = Some((cid, span.0, span.1, span.2, span.3, len)),
					Some((_, _, _, _, _, bl)) if len > bl => {
						best = Some((cid, span.0, span.1, span.2, span.3, len));
					}
					_ => {}
				}
			}
		}
	}
	let (cell_id, along_x, lo, hi, mid, shared_len) = best?;
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
		OpeningId::scoped(SCOPE, "hall_door", group_id.to_string()),
		Opening::new(bounds, OpeningLabel::Passage),
		cell_id,
	))
}

/// `(along_x, lo_along, hi_along, mid_perp)`.
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

/// Walls only where apartments meet each other or a hall (no outer / no shells).
fn enclosure_walls(
	cells: &[PlanCell],
	halls: &[Aabb2d],
	group_of: &std::collections::HashMap<u32, usize>,
	openings: &Openings,
	y0: f32,
	y1: f32,
) -> Vec<RectFloor> {
	let thickness = DEFAULT_PANEL_THICKNESS.max(0.12);
	let height = (y1 - y0).max(2.0);
	let mut walls = Vec::new();

	// Partition walls between different apartment groups.
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
			if let Some(wall) = wall_on_shared_edge(a.bounds, b.bounds, openings, y0, height, thickness)
			{
				walls.push(wall);
			}
		}
	}

	// Hall-frontage walls (one wall per cell–hall shared edge).
	for cell in cells {
		if !group_of.contains_key(&cell.id) {
			continue;
		}
		for hall in halls {
			if let Some(wall) =
				wall_on_shared_edge(cell.bounds, *hall, openings, y0, height, thickness)
			{
				walls.push(wall);
			}
		}
	}

	walls
}

fn wall_on_shared_edge(
	a: Aabb2d,
	b: Aabb2d,
	openings: &Openings,
	y0: f32,
	height: f32,
	thickness: f32,
) -> Option<RectFloor> {
	let (along_x, lo, hi, mid) = shared_edge_span(a, b)?;
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
	Some(RectFloor::new(RectFloorParams {
		center_xz,
		footprint,
		storey_height: height,
		openings: wall_openings,
		floor: RectFloorSlab::None,
		ceiling: RectFloorSlab::None,
		..RectFloorParams::default()
	}))
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
				targets: Some(vec![40.0, 30.0, 22.0, 18.0]),
			},
		)
		.unwrap();
		assert!(!block.halls.hall_bands.is_empty());
		assert!(!block.apartments.is_empty());
		assert!(block.apartments.iter().all(|a| a.shell.is_none()));
		// At most one hall door per apartment group.
		let door_count = block
			.apartments
			.iter()
			.flat_map(|a| a.cells.iter())
			.flat_map(|p| p.confines.openings.iter())
			.filter(|(id, _)| id.as_str().contains("hall_door"))
			.count();
		assert!(door_count >= 1, "expected hall doors on groups");
		assert!(
			door_count <= block.apartments.len(),
			"expected ≤1 door per apartment, doors={door_count} apts={}",
			block.apartments.len()
		);
		let _ = regions;
	}

	#[test]
	fn packs_some_multi_cell_apartments() {
		let confines = host_with_shafts_and_passage();
		let (block, _) = LivableApartments::from_confines_with(
			&confines,
			NoiseParams { seed: 3, ..NoiseParams::default() },
			LivableApartmentsOptions {
				hall_width: Some(2.5),
				targets: Some(vec![55.0, 48.0, 40.0, 30.0]),
			},
		)
		.unwrap();
		assert!(
			block.apartments.iter().any(|a| a.cells.len() >= 2),
			"expected at least one non-rectangular / multi-cell group"
		);
	}
}
