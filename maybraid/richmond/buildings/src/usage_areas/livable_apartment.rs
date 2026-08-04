//! Livable apartment group: multi-cell envelope filled with living quarters.
//!
//! First-cut fill: each residual cell is offered to a catalog of inbound
//! quarters ([`CommonBedroom`], [`LivingRoom`], [`Kitchen`], …) in first-fit
//! order. Adjacent filled cells get a connecting passage + partition wall.
//! Cells that reject every quarter become [`SpaceKind::ClosetSpace`] residuals.

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseParams, NoiseType, TypedBucketThrow};
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
use crate::usage_areas::livable_quarters::{
	DiningRoom, Kitchen, LivingRoom, ResidentialBathroom, ResidentialHalfBathroom, SittingRoom,
	Study,
};
use crate::usage_areas::plan_cells::{cells_edge_adjacent, PlanCell};

const EPS: f32 = 1e-3;
const DOOR_WIDTH: f32 = 1.0;
const SCOPE: &str = "livable_apartment";

/// One packed living-quarter room inside an apartment.
#[derive(Debug, Clone, PartialEq)]
pub enum ApartmentRoom {
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

const FIRST_FIT_ORDER: &[QuarterKind] = &[
	QuarterKind::Bedroom,
	QuarterKind::Living,
	QuarterKind::Kitchen,
	QuarterKind::Dining,
	QuarterKind::Bathroom,
	QuarterKind::Sitting,
	QuarterKind::Study,
	QuarterKind::HalfBath,
];

fn quarter_catalog() -> TypedBucketThrow<QuarterKind> {
	let mut d = TypedBucketThrow::new();
	d.add(QuarterKind::Bedroom, 3.0);
	d.add(QuarterKind::Living, 2.5);
	d.add(QuarterKind::Kitchen, 2.0);
	d.add(QuarterKind::Dining, 1.5);
	d.add(QuarterKind::Bathroom, 1.5);
	d.add(QuarterKind::Sitting, 1.2);
	d.add(QuarterKind::Study, 1.0);
	d.add(QuarterKind::HalfBath, 0.8);
	d
}

/// One apartment group: envelope cells + packed living quarters.
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartment {
	pub region_id: u32,
	/// Room cells that make up this apartment (often a single rectangle).
	pub cells: MultiConfines,
	/// Packed living quarters (first-cut catalog fill).
	pub rooms: Vec<ApartmentRoom>,
	/// Partition strips between packed rooms (with connecting passages).
	pub partitions: Vec<ClippedRectangularStrip>,
	/// Optional envelope shell for the primary / first cell (presentation).
	pub shell: Option<RectFloor>,
}

impl LivableApartment {
	/// Single-cell convenience (one-part [`MultiConfines`]).
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

	/// Multi-cell apartment group with first-cut living-quarters fill.
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

		let mut rooms = Vec::new();
		let mut residual_within = Vec::new();
		let mut filled_cells: Vec<(usize, Confines)> = Vec::new();

		let preferred = pick_preferred_kind(noise, cells.parts[0].confines.center());
		for (ci, part) in cells.iter().enumerate() {
			let cell_noise = noise_for_cell(noise, ci as i32);
			match pack_quarter_into_cell(&part.confines, cell_noise, preferred) {
				Ok((room, nested)) => {
					rooms.push(room);
					filled_cells.push((ci, part.confines.clone()));
					residual_within.extend(nested.within);
				}
				Err(FitError::TooSmall { .. }) => {
					// Unfilled pocket → closet residual for Full* to consume.
					residual_within.push(FillRegion::new(
						SpaceKind::ClosetSpace,
						part.confines.clone(),
					));
				}
				Err(err) => return Err(err),
			}
		}

		if rooms.is_empty() {
			return Err(FitError::TooSmall {
				reason: "livable_no_quarters",
			});
		}

		let (partitions, connect_openings) =
			connect_filled_cells(&filled_cells, region_id);
		// Attach connecting passages onto residual closet/room confines is deferred;
		// openings are punched into the partition strips themselves.
		let _ = connect_openings;

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

	/// Primary confines (first cell) — useful for labels / single-cell callers.
	pub fn primary_confines(&self) -> &Confines {
		&self.cells.parts[0].confines
	}
}

fn pick_preferred_kind(noise: NoiseParams, center: Vec3) -> QuarterKind {
	let catalog = quarter_catalog();
	let n = NoiseParams {
		noise_type: NoiseType::Cellular,
		frequency: 0.35,
		..noise
	};
	catalog
		.select_from_noise_3d(n, center)
		.copied()
		.unwrap_or(QuarterKind::Living)
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
	let mut order = vec![preferred];
	for &k in FIRST_FIT_ORDER {
		if k != preferred {
			order.push(k);
		}
	}
	let mut last_err = FitError::TooSmall {
		reason: "livable_quarter",
	};
	for kind in order {
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

/// Partition + door between every pair of edge-adjacent filled cells.
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
				if let Some(wall) = partition_strip(
					along_x,
					lo,
					hi,
					mid,
					y0,
					height,
					thickness,
					&Openings::new().with(id, opening),
				) {
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
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fills_rectangle_with_a_quarter() {
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(8.0, 3.0, 6.0)),
			0.0,
			Openings::new(),
		);
		let (apt, regions) =
			LivableApartment::from_confines(0, &confines, NoiseParams::default()).unwrap();
		assert!(!apt.rooms.is_empty());
		assert_eq!(apt.cells.len(), 1);
		// Unfilled leftovers (if any) should be closet-typed.
		assert!(regions
			.within
			.iter()
			.all(|r| matches!(
				r.kind,
				SpaceKind::ClosetSpace | SpaceKind::InternalSpace | SpaceKind::Custom(_)
			)));
	}

	#[test]
	fn fills_multi_cell() {
		let a = Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 3.0, 5.0)),
			0.0,
			Openings::new(),
		);
		let b = Confines::new(
			Aabb3d::from_min_max(Vec3::new(5.0, 0.0, 0.0), Vec3::new(10.0, 3.0, 5.0)),
			0.0,
			Openings::new(),
		);
		let multi = MultiConfines::new([
			FillRegion::new(SpaceKind::InternalSpace, a),
			FillRegion::new(SpaceKind::InternalSpace, b),
		]);
		let (apt, _) =
			LivableApartment::from_multi(0, &multi, NoiseParams::default()).unwrap();
		assert_eq!(apt.cells.len(), 2);
		assert!(!apt.rooms.is_empty());
	}
}
