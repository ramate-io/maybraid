//! Rectangular livable area: open/closed packing inside one axis-aligned rect.
//!
//! Guarantees (normalize):
//! 1. Every inbound passage sits on an **open** face ≥ [`min_hall`](RectangularLivableAreaParameterized::min_hall).
//! 2. Open regions form one connected component touching all passages.
//! 3. Every closed region has ≥1 door onto that open component.
//!
//! Strategies: [`RectLivableStrategy`] — default [`CaseAttempt`](RectLivableStrategy::CaseAttempt).

mod parameterized;

pub use parameterized::{
	RectLivableStrategy, RectangularLivableAreaParameterized, RectangularLivableAreaPlan, SCOPE,
	DEFAULT_CLOSED_MAX_AREA, DEFAULT_MIN_HALL,
};

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::{aabb2_area, Aabb2dPack, NoiseParams};
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::{LabelNode, LabelStyle};
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::rect_fit::RectInset;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::shells::ortho::{standing_face_opening, WallEdge};
use crate::usage_areas::common_bedroom::CommonBedroom;
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::livable_quarters::{
	DiningRoom, EatingArea, Kitchen, LivingRoom, ResidentialBathroom, ResidentialHalfBathroom,
	SittingRoom, Study,
};
use crate::usage_areas::plan_cells::{shared_edge_span, subtract_aabb2};

const EPS: f32 = 1e-3;
const DOOR_WIDTH: f32 = 1.0;
const MIN_ROOM: f32 = 2.2;

/// Preferred quarter kinds for a program slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectQuarterKind {
	Bedroom,
	Living,
	/// Kitchen + dining side-by-side (falls back to kitchen-only).
	Eating,
	/// Leaf kitchen (prefer [`Eating`] in programs).
	Kitchen,
	/// Leaf dining (prefer [`Eating`] in programs).
	Dining,
	Bathroom,
	HalfBath,
	Sitting,
	Study,
}

impl RectQuarterKind {
	pub fn is_closed(self) -> bool {
		matches!(
			self,
			Self::Bedroom | Self::Bathroom | Self::HalfBath | Self::Study
		)
	}

	pub fn is_open(self) -> bool {
		!self.is_closed()
	}
}

/// One packed space inside a rectangular livable area.
#[derive(Debug, Clone, PartialEq)]
pub enum RectAreaRoom {
	/// Unwalled hall / open remnant (circulation).
	OpenBand {
		label: LabelNode,
		confines: Confines,
	},
	HouseholdCloset {
		label: LabelNode,
		confines: Confines,
	},
	Bedroom(CommonBedroom),
	Living(LivingRoom),
	Eating(EatingArea),
	Kitchen(Kitchen),
	Dining(DiningRoom),
	Bathroom(ResidentialBathroom),
	HalfBath(ResidentialHalfBathroom),
	Sitting(SittingRoom),
	Study(Study),
}

impl RectAreaRoom {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::OpenBand { .. } | Self::HouseholdCloset { .. } => Layers::new(),
			Self::Bedroom(r) => r.panel_nodes_for_level(level),
			Self::Living(r) => r.panel_nodes_for_level(level),
			Self::Eating(r) => r.panel_nodes_for_level(level),
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
			Self::OpenBand { .. } | Self::HouseholdCloset { .. } => Layers::new(),
			Self::Bedroom(r) => r.joint_nodes_for_level(level),
			Self::Living(r) => r.joint_nodes_for_level(level),
			Self::Eating(r) => r.joint_nodes_for_level(level),
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
			Self::OpenBand { label, .. } | Self::HouseholdCloset { label, .. } => {
				let mut out = Layers::new();
				out.push_free(label.clone());
				out
			}
			Self::Bedroom(r) => r.label_nodes_for_level(level),
			Self::Living(r) => r.label_nodes_for_level(level),
			Self::Eating(r) => r.label_nodes_for_level(level),
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
			Self::OpenBand { .. } | Self::HouseholdCloset { .. } => Layers::new(),
			Self::Bedroom(r) => r.furniture_nodes_for_level(level),
			Self::Living(r) => r.furniture_nodes_for_level(level),
			Self::Eating(r) => r.furniture_nodes_for_level(level),
			Self::Kitchen(r) => r.furniture_nodes_for_level(level),
			Self::Dining(r) => r.furniture_nodes_for_level(level),
			Self::Bathroom(r) => r.furniture_nodes_for_level(level),
			Self::HalfBath(r) => r.furniture_nodes_for_level(level),
			Self::Sitting(r) => r.furniture_nodes_for_level(level),
			Self::Study(r) => r.furniture_nodes_for_level(level),
		}
	}
}

/// Fitted rectangular livable area (rooms + local hall bands + partitions).
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularLivableArea {
	pub confines: Confines,
	pub rooms: Vec<RectAreaRoom>,
	/// Open circulation bands (identification / normalize).
	pub walkways: Vec<Aabb2d>,
	pub partitions: Vec<ClippedRectangularStrip>,
	pub plan: RectangularLivableAreaPlan,
	/// Closed-room confines that need apartment-style walls.
	pub(crate) closed_confines: Vec<Confines>,
	/// Open-room / band confines (for normalize + apartment aggregate).
	pub(crate) open_confines: Vec<Confines>,
}

impl RectangularLivableArea {
	/// Fit with explicit params and a preferred program slice.
	pub fn fit_with_params(
		confines: &Confines,
		noise: NoiseParams,
		params: RectangularLivableAreaParameterized,
		program: &[RectQuarterKind],
	) -> Result<(Self, FillableRegions), FitError> {
		let fp = confines.footprint();
		if fp.x < params.min_hall || fp.y < params.min_hall {
			return Err(FitError::TooSmall {
				reason: "rla_footprint",
			});
		}
		let strategies = strategy_order(params.strategy, program);
		let mut last = FitError::TooSmall {
			reason: "rla_exhausted",
		};
		for strategy in strategies {
			match try_strategy(confines, noise, params, program, strategy) {
				Ok(ok) => return Ok(ok),
				Err(FitError::TooSmall { reason }) => {
					last = FitError::TooSmall { reason };
				}
				Err(err) => return Err(err),
			}
		}
		Err(last)
	}
}

impl Fit for RectangularLivableArea {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = RectangularLivableAreaParameterized::sample(confines, noise)?;
		let area = {
			let fp = confines.footprint();
			fp.x * fp.y
		};
		let program = default_program(area, passage_count(confines));
		Self::fit_with_params(confines, noise, params, &program)
	}
}

impl BuildingComponents for RectangularLivableArea {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
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
		out.push_free(label_filling_aabb(
			LabelStyle::Blue,
			"RectLivable",
			&self.confines.bounds,
			self.confines.roll,
		));
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

fn strategy_order(
	requested: RectLivableStrategy,
	program: &[RectQuarterKind],
) -> Vec<RectLivableStrategy> {
	match requested {
		RectLivableStrategy::CaseAttempt => {
			if program.iter().any(|k| k.is_closed()) {
				vec![
					RectLivableStrategy::SpineHall,
					RectLivableStrategy::GuillotineSplit,
					RectLivableStrategy::SingleClosed,
					RectLivableStrategy::AllOpen,
				]
			} else {
				vec![
					RectLivableStrategy::AllOpen,
					RectLivableStrategy::SingleClosed,
					RectLivableStrategy::SpineHall,
					RectLivableStrategy::GuillotineSplit,
				]
			}
		}
		other => vec![other],
	}
}

fn default_program(area: f32, passages: usize) -> Vec<RectQuarterKind> {
	let mut out = Vec::new();
	if passages == 1 && area <= DEFAULT_CLOSED_MAX_AREA {
		out.push(RectQuarterKind::Bedroom);
		return out;
	}
	// Eating before living so kitchens claim free space first when packing.
	if area + EPS >= min_area_for(RectQuarterKind::Eating) {
		out.push(RectQuarterKind::Eating);
	}
	out.push(RectQuarterKind::Living);
	if area > 28.0 {
		out.push(RectQuarterKind::Bedroom);
	}
	if area > 40.0 {
		out.push(RectQuarterKind::Bathroom);
	}
	out
}

fn passage_count(confines: &Confines) -> usize {
	confines
		.openings
		.iter()
		.filter(|(_, o)| matches!(o.label, OpeningLabel::Passage))
		.count()
}

fn try_strategy(
	confines: &Confines,
	noise: NoiseParams,
	params: RectangularLivableAreaParameterized,
	program: &[RectQuarterKind],
	strategy: RectLivableStrategy,
) -> Result<(RectangularLivableArea, FillableRegions), FitError> {
	let (mut area, residual) = match strategy {
		RectLivableStrategy::CaseAttempt => {
			return Err(FitError::TooSmall {
				reason: "rla_case_attempt_leaf",
			});
		}
		RectLivableStrategy::AllOpen => fit_all_open(confines, noise, params, program)?,
		RectLivableStrategy::SingleClosed => fit_single_closed(confines, noise, params, program)?,
		RectLivableStrategy::SpineHall => fit_spine_hall(confines, noise, params, program)?,
		RectLivableStrategy::GuillotineSplit => {
			fit_guillotine(confines, noise, params, program)?
		}
	};
	if !normalize_ok(&area, params.min_hall) {
		return Err(FitError::TooSmall {
			reason: "rla_normalize",
		});
	}
	area.plan.chosen = strategy;
	Ok((area, residual))
}

fn fit_all_open(
	confines: &Confines,
	noise: NoiseParams,
	params: RectangularLivableAreaParameterized,
	program: &[RectQuarterKind],
) -> Result<(RectangularLivableArea, FillableRegions), FitError> {
	let open_kinds: Vec<_> = program.iter().copied().filter(|k| k.is_open()).collect();
	let kinds = if open_kinds.is_empty() {
		vec![RectQuarterKind::Living]
	} else {
		open_kinds
	};
	let host = host_xz(&confines.bounds);
	let y0 = Vec3::from(confines.bounds.min).y;
	let y1 = Vec3::from(confines.bounds.max).y;
	let roll = confines.roll;

	// Single open kind → claim the whole rect (circulation = host).
	if kinds.len() == 1 {
		match try_fit_kind(kinds[0], confines, noise) {
			Ok((room, nested)) => {
				return Ok((
					RectangularLivableArea {
						confines: confines.clone(),
						rooms: vec![room],
						walkways: vec![host],
						partitions: Vec::new(),
						plan: RectangularLivableAreaPlan {
							parameterized: params,
							chosen: RectLivableStrategy::AllOpen,
							hall_bands: vec![host],
						},
						closed_confines: Vec::new(),
						open_confines: vec![confines.clone()],
					},
					nested,
				));
			}
			Err(FitError::TooSmall { .. }) => {}
			Err(err) => return Err(err),
		}
	}

	// Multiple open kinds: partition free space (no walls) so kitchen/dining/
	// sitting get labeled rooms instead of one living over the whole footprint.
	let mut free = vec![host];
	let mut rooms = Vec::new();
	let mut open_confines = Vec::new();
	let mut residual_within = Vec::new();
	pack_open_into(
		&mut free,
		&kinds,
		None,
		y0,
		y1,
		roll,
		noise,
		&mut rooms,
		&mut open_confines,
		&mut residual_within,
	)?;
	for scrap in free {
		let c = confines_from_xz(scrap, y0, y1, roll, &Openings::new());
		push_leftover(&mut rooms, &mut residual_within, c);
	}
	if rooms.is_empty() {
		rooms.push(RectAreaRoom::OpenBand {
			label: label_filling_aabb(
				LabelStyle::Cyan,
				"OpenHall",
				&confines.bounds,
				roll,
			),
			confines: confines.clone(),
		});
		open_confines.push(confines.clone());
	}
	Ok((
		RectangularLivableArea {
			confines: confines.clone(),
			rooms,
			walkways: vec![host],
			partitions: Vec::new(),
			plan: RectangularLivableAreaPlan {
				parameterized: params,
				chosen: RectLivableStrategy::AllOpen,
				hall_bands: vec![host],
			},
			closed_confines: Vec::new(),
			open_confines,
		},
		FillableRegions {
			within: residual_within,
			atop: Vec::new(),
		},
	))
}

fn fit_single_closed(
	confines: &Confines,
	noise: NoiseParams,
	params: RectangularLivableAreaParameterized,
	program: &[RectQuarterKind],
) -> Result<(RectangularLivableArea, FillableRegions), FitError> {
	if passage_count(confines) != 1 {
		return Err(FitError::TooSmall {
			reason: "rla_single_closed_ports",
		});
	}
	let area_m2 = {
		let fp = confines.footprint();
		fp.x * fp.y
	};
	if area_m2 > params.closed_max_area + EPS {
		return Err(FitError::TooSmall {
			reason: "rla_single_closed_area",
		});
	}
	let closed_kinds: Vec<_> = program.iter().copied().filter(|k| k.is_closed()).collect();
	let kinds = if closed_kinds.is_empty() {
		vec![
			RectQuarterKind::Bedroom,
			RectQuarterKind::Bathroom,
			RectQuarterKind::Study,
		]
	} else {
		closed_kinds
	};
	let mut last = FitError::TooSmall {
		reason: "rla_single_closed",
	};
	for &kind in &kinds {
		match try_fit_kind(kind, confines, noise) {
			Ok((room, nested)) => {
				let host = host_xz(&confines.bounds);
				// Closed room needs a zero-area "open" witness at the passage face
				// for normalize — use a thin open band on the door edge.
				let Some(open_band) = passage_lip_band(confines, params.min_hall) else {
					last = FitError::TooSmall {
						reason: "rla_single_closed_lip",
					};
					continue;
				};
				let y0 = Vec3::from(confines.bounds.min).y;
				let y1 = Vec3::from(confines.bounds.max).y;
				let open_c = confines_from_xz(open_band, y0, y1, confines.roll, &Openings::new());
				let open_room = RectAreaRoom::OpenBand {
					label: label_filling_aabb(
						LabelStyle::Cyan,
						"DoorClear",
						&open_c.bounds,
						confines.roll,
					),
					confines: open_c.clone(),
				};
				let partitions =
					enclose_closed_rooms(&[confines.clone()], &[open_band], 0);
				let area = RectangularLivableArea {
					confines: confines.clone(),
					rooms: vec![room, open_room],
					walkways: vec![open_band],
					partitions,
					plan: RectangularLivableAreaPlan {
						parameterized: params,
						chosen: RectLivableStrategy::SingleClosed,
						hall_bands: vec![open_band],
					},
					closed_confines: vec![confines.clone()],
					open_confines: vec![open_c],
				};
				let _ = host;
				return Ok((area, nested));
			}
			Err(FitError::TooSmall { reason }) => {
				last = FitError::TooSmall { reason };
			}
			Err(err) => return Err(err),
		}
	}
	Err(last)
}

fn fit_spine_hall(
	confines: &Confines,
	noise: NoiseParams,
	params: RectangularLivableAreaParameterized,
	program: &[RectQuarterKind],
) -> Result<(RectangularLivableArea, FillableRegions), FitError> {
	let host = host_xz(&confines.bounds);
	let y0 = Vec3::from(confines.bounds.min).y;
	let y1 = Vec3::from(confines.bounds.max).y;
	let roll = confines.roll;
	let min_hall = params.min_hall;

	let spine = build_spine_connecting_passages(confines, min_hall).ok_or(FitError::TooSmall {
		reason: "rla_spine",
	})?;
	if spine.is_empty() {
		return Err(FitError::TooSmall {
			reason: "rla_spine_empty",
		});
	}

	let mut rooms = Vec::new();
	let mut residual_within = Vec::new();
	let mut closed_confines = Vec::new();
	let mut open_confines = Vec::new();
	let mut filled_closed: Vec<Confines> = Vec::new();

	// Open spine bands.
	for (i, band) in spine.iter().enumerate() {
		let c = confines_from_xz(*band, y0, y1, roll, &Openings::new());
		rooms.push(RectAreaRoom::OpenBand {
			label: label_filling_aabb(
				LabelStyle::Cyan,
				&format!("Hall{i}"),
				&c.bounds,
				roll,
			),
			confines: c.clone(),
		});
		open_confines.push(c);
	}

	// Private pockets: host − spine; must abut spine ≥ min_hall.
	let mut private_free = subtract_aabb2(host, &spine);
	private_free.retain(|r| rect_usable(*r, 3.0));
	let closed_kinds: Vec<_> = program.iter().copied().filter(|k| k.is_closed()).collect();
	pack_closed_abutting(
		&mut private_free,
		&closed_kinds,
		&spine,
		min_hall,
		y0,
		y1,
		roll,
		noise,
		&mut rooms,
		&mut closed_confines,
		&mut filled_closed,
		&mut residual_within,
	)?;

	// Common: residual including spine (open rooms may overlap halls).
	let occupied: Vec<Aabb2d> = filled_closed.iter().map(|c| host_xz(&c.bounds)).collect();
	let mut common_free = subtract_aabb2(host, &occupied);
	common_free.retain(|r| rect_usable(*r, 4.0) || aabb2_area(*r) > 6.0);
	let open_kinds: Vec<_> = program.iter().copied().filter(|k| k.is_open()).collect();
	pack_open_into(
		&mut common_free,
		&open_kinds,
		Some(&spine),
		y0,
		y1,
		roll,
		noise,
		&mut rooms,
		&mut open_confines,
		&mut residual_within,
	)?;

	for scrap in common_free.into_iter().chain(private_free) {
		let c = confines_from_xz(scrap, y0, y1, roll, &Openings::new());
		push_leftover(&mut rooms, &mut residual_within, c);
	}

	let partitions = enclose_closed_rooms(&filled_closed, &spine, 0);
	Ok((
		RectangularLivableArea {
			confines: confines.clone(),
			rooms,
			walkways: spine.clone(),
			partitions,
			plan: RectangularLivableAreaPlan {
				parameterized: params,
				chosen: RectLivableStrategy::SpineHall,
				hall_bands: spine,
			},
			closed_confines,
			open_confines,
		},
		FillableRegions {
			within: residual_within,
			atop: Vec::new(),
		},
	))
}

fn fit_guillotine(
	confines: &Confines,
	noise: NoiseParams,
	params: RectangularLivableAreaParameterized,
	program: &[RectQuarterKind],
) -> Result<(RectangularLivableArea, FillableRegions), FitError> {
	let host = host_xz(&confines.bounds);
	let size = host.max - host.min;
	if size.x < params.min_hall * 2.5 && size.y < params.min_hall * 2.5 {
		return Err(FitError::TooSmall {
			reason: "rla_guillotine_small",
		});
	}
	let y0 = Vec3::from(confines.bounds.min).y;
	let y1 = Vec3::from(confines.bounds.max).y;
	let roll = confines.roll;
	let split_x = size.x >= size.y;
	let frac = 0.45;
	let (a, b) = if split_x {
		host.bipartition_by_area(true, true, frac)
	} else {
		host.bipartition_by_area(false, true, frac)
	};
	if !rect_usable(a, 6.0) || !rect_usable(b, 6.0) {
		return Err(FitError::TooSmall {
			reason: "rla_guillotine_parts",
		});
	}

	let Some((along_x, lo, hi, mid)) = shared_edge_span(a, b) else {
		return Err(FitError::TooSmall {
			reason: "rla_guillotine_edge",
		});
	};
	if hi - lo + EPS < params.min_hall {
		return Err(FitError::TooSmall {
			reason: "rla_guillotine_access",
		});
	}
	let passage = connecting_passage(along_x, lo, hi, mid, y0, y1, 0, 0, 1).ok_or(
		FitError::TooSmall {
			reason: "rla_guillotine_door",
		},
	)?;

	let (open_a, open_b) = split_host_openings(confines, a, b);
	let mut openings_a = open_a;
	let mut openings_b = open_b;
	openings_a.insert(passage.0.clone(), passage.1.clone());
	openings_b.insert(passage.0, passage.1);

	let confines_a = confines_from_xz(a, y0, y1, roll, &openings_a);
	let confines_b = confines_from_xz(b, y0, y1, roll, &openings_b);

	let (prog_a, prog_b) = split_program(program);
	let child_params = RectangularLivableAreaParameterized {
		strategy: RectLivableStrategy::CaseAttempt,
		..params
	};
	let (child_a, res_a) = RectangularLivableArea::fit_with_params(
		&confines_a,
		noise_for_cell(noise, 11),
		child_params,
		&prog_a,
	)?;
	let (child_b, res_b) = RectangularLivableArea::fit_with_params(
		&confines_b,
		noise_for_cell(noise, 29),
		child_params,
		&prog_b,
	)?;

	let mut rooms = child_a.rooms;
	rooms.extend(child_b.rooms);
	let mut walkways = child_a.walkways;
	walkways.extend(child_b.walkways);
	let mut partitions = child_a.partitions;
	partitions.extend(child_b.partitions);
	let mut closed_confines = child_a.closed_confines;
	closed_confines.extend(child_b.closed_confines);
	let mut open_confines = child_a.open_confines;
	open_confines.extend(child_b.open_confines);
	let mut residual = res_a;
	residual.within.extend(res_b.within);

	Ok((
		RectangularLivableArea {
			confines: confines.clone(),
			rooms,
			walkways: walkways.clone(),
			partitions,
			plan: RectangularLivableAreaPlan {
				parameterized: params,
				chosen: RectLivableStrategy::GuillotineSplit,
				hall_bands: walkways,
			},
			closed_confines,
			open_confines,
		},
		residual,
	))
}

/// Normalize: passages on open ≥ min_hall; open connected; closed doors onto open.
pub fn normalize_ok(area: &RectangularLivableArea, min_hall: f32) -> bool {
	let open_rects: Vec<Aabb2d> = area
		.open_confines
		.iter()
		.map(|c| host_xz(&c.bounds))
		.chain(area.walkways.iter().copied())
		.collect();
	if open_rects.is_empty() {
		return false;
	}
	// Dedup-ish: keep as-is; connectivity uses shared edges.
	let passages: Vec<_> = area
		.confines
		.openings
		.iter()
		.filter(|(_, o)| matches!(o.label, OpeningLabel::Passage))
		.map(|(_, o)| o.clone())
		.collect();
	if passages.is_empty() {
		// Interior leaf without ports — allow if we still have open space.
		return open_connected(&open_rects, min_hall);
	}
	for p in &passages {
		if passage_open_overlap(p, &open_rects, &area.confines, min_hall) < min_hall - EPS {
			return false;
		}
	}
	if !open_connected(&open_rects, min_hall) {
		return false;
	}
	for closed in &area.closed_confines {
		let cz = host_xz(&closed.bounds);
		let touches_open = open_rects.iter().any(|o| {
			shared_edge_span(cz, *o).is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_hall)
				|| overlap_area_proxy(cz, &[*o]) + EPS >= min_hall
				|| rect_covers_edge(cz, *o, min_hall)
		});
		if !touches_open {
			return false;
		}
	}
	true
}

/// True when `inner` lies on a face of `outer` with contact length ≥ `min_hall`.
fn rect_covers_edge(outer: Aabb2d, inner: Aabb2d, min_hall: f32) -> bool {
	let ix0 = inner.min.x.max(outer.min.x);
	let ix1 = inner.max.x.min(outer.max.x);
	let iy0 = inner.min.y.max(outer.min.y);
	let iy1 = inner.max.y.min(outer.max.y);
	if ix1 - ix0 <= EPS || iy1 - iy0 <= EPS {
		return false;
	}
	let on_w = (inner.min.x - outer.min.x).abs() < 0.15;
	let on_e = (inner.max.x - outer.max.x).abs() < 0.15;
	let on_s = (inner.min.y - outer.min.y).abs() < 0.15;
	let on_n = (inner.max.y - outer.max.y).abs() < 0.15;
	if (on_w || on_e) && iy1 - iy0 + EPS >= min_hall {
		return true;
	}
	if (on_s || on_n) && ix1 - ix0 + EPS >= min_hall {
		return true;
	}
	false
}

fn passage_open_overlap(
	opening: &Opening,
	open_rects: &[Aabb2d],
	host: &Confines,
	min_hall: f32,
) -> f32 {
	let _ = min_hall;
	let host_xz = host_xz(&host.bounds);
	let dmin = Vec3::from(opening.bounds.min);
	let dmax = Vec3::from(opening.bounds.max);
	let door_xz = Aabb2d {
		min: Vec2::new(dmin.x.min(dmax.x), dmin.z.min(dmax.z)),
		max: Vec2::new(dmin.x.max(dmax.x), dmin.z.max(dmax.z)),
	};
	// Expand door slightly toward host interior for edge contact.
	let inflated = inflate_toward_host(door_xz, host_xz, 0.2);
	open_rects
		.iter()
		.filter_map(|o| shared_edge_span(inflated, *o).or_else(|| aabb2_overlap_len(inflated, *o)))
		.map(|span| match span {
			(true, lo, hi, _) | (false, lo, hi, _) => hi - lo,
		})
		.fold(0.0_f32, f32::max)
		.max(overlap_area_proxy(inflated, open_rects))
}

fn aabb2_overlap_len(a: Aabb2d, b: Aabb2d) -> Option<(bool, f32, f32, f32)> {
	let x0 = a.min.x.max(b.min.x);
	let x1 = a.max.x.min(b.max.x);
	let y0 = a.min.y.max(b.min.y);
	let y1 = a.max.y.min(b.max.y);
	if x1 - x0 > EPS && y1 - y0 > EPS {
		// Treat as along the longer overlap axis.
		if x1 - x0 >= y1 - y0 {
			return Some((true, x0, x1, 0.5 * (y0 + y1)));
		}
		return Some((false, y0, y1, 0.5 * (x0 + x1)));
	}
	shared_edge_span(a, b)
}

fn overlap_area_proxy(door: Aabb2d, opens: &[Aabb2d]) -> f32 {
	opens
		.iter()
		.map(|o| {
			let x0 = door.min.x.max(o.min.x);
			let x1 = door.max.x.min(o.max.x);
			let y0 = door.min.y.max(o.min.y);
			let y1 = door.max.y.min(o.max.y);
			((x1 - x0).max(0.0) * (y1 - y0).max(0.0)).sqrt()
		})
		.fold(0.0_f32, f32::max)
}

fn inflate_toward_host(door: Aabb2d, host: Aabb2d, d: f32) -> Aabb2d {
	let mut r = door;
	if (door.min.x - host.min.x).abs() < 0.25 {
		r.max.x = (r.max.x + d).min(host.max.x);
	} else if (door.max.x - host.max.x).abs() < 0.25 {
		r.min.x = (r.min.x - d).max(host.min.x);
	} else if (door.min.y - host.min.y).abs() < 0.25 {
		r.max.y = (r.max.y + d).min(host.max.y);
	} else if (door.max.y - host.max.y).abs() < 0.25 {
		r.min.y = (r.min.y - d).max(host.min.y);
	} else {
		r.min -= Vec2::splat(d * 0.5);
		r.max += Vec2::splat(d * 0.5);
		r.min = r.min.max(host.min);
		r.max = r.max.min(host.max);
	}
	r
}

fn open_connected(rects: &[Aabb2d], min_hall: f32) -> bool {
	if rects.is_empty() {
		return false;
	}
	let n = rects.len();
	let mut seen = vec![false; n];
	let mut stack = vec![0usize];
	seen[0] = true;
	while let Some(i) = stack.pop() {
		for j in 0..n {
			if seen[j] {
				continue;
			}
			let touch = shared_edge_span(rects[i], rects[j])
				.is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_hall * 0.5)
				|| overlap_area_proxy(rects[i], &[rects[j]]) > EPS;
			if touch {
				seen[j] = true;
				stack.push(j);
			}
		}
	}
	seen.into_iter().all(|s| s)
}

fn build_spine_connecting_passages(confines: &Confines, min_hall: f32) -> Option<Vec<Aabb2d>> {
	let host = host_xz(&confines.bounds);
	let ports = passage_port_points(confines, &host);
	if ports.is_empty() {
		// No passages — central corridor strip.
		return Some(vec![central_band(host, min_hall)]);
	}
	if ports.len() == 1 {
		return Some(vec![band_from_port(host, ports[0], min_hall)]);
	}
	// Axis-aligned Steiner: bands from each port to a center spine.
	let cx = 0.5 * (host.min.x + host.max.x);
	let cy = 0.5 * (host.min.y + host.max.y);
	let half = min_hall * 0.5;
	let size = host.max - host.min;
	let mut bands = Vec::new();
	if size.x >= size.y {
		// Horizontal main spine through center.
		let y0 = (cy - half).clamp(host.min.y, host.max.y - min_hall);
		bands.push(Aabb2d {
			min: Vec2::new(host.min.x, y0),
			max: Vec2::new(host.max.x, y0 + min_hall),
		});
	} else {
		let x0 = (cx - half).clamp(host.min.x, host.max.x - min_hall);
		bands.push(Aabb2d {
			min: Vec2::new(x0, host.min.y),
			max: Vec2::new(x0 + min_hall, host.max.y),
		});
	}
	for p in ports {
		bands.push(band_from_port(host, p, min_hall));
	}
	// Clip bands to host (already inside).
	bands.retain(|b| aabb2_area(*b) > EPS);
	Some(bands)
}

fn passage_port_points(confines: &Confines, host: &Aabb2d) -> Vec<Vec2> {
	let mut out = Vec::new();
	for (_, o) in confines.openings.iter() {
		if !matches!(o.label, OpeningLabel::Passage) {
			continue;
		}
		let dmin = Vec3::from(o.bounds.min);
		let dmax = Vec3::from(o.bounds.max);
		let c = Vec2::new(0.5 * (dmin.x + dmax.x), 0.5 * (dmin.z + dmax.z));
		let clamped = Vec2::new(
			c.x.clamp(host.min.x, host.max.x),
			c.y.clamp(host.min.y, host.max.y),
		);
		out.push(clamped);
	}
	out
}

fn central_band(host: Aabb2d, w: f32) -> Aabb2d {
	let size = host.max - host.min;
	let half = w * 0.5;
	if size.x >= size.y {
		let cy = 0.5 * (host.min.y + host.max.y);
		let y0 = (cy - half).clamp(host.min.y, host.max.y - w);
		Aabb2d {
			min: Vec2::new(host.min.x, y0),
			max: Vec2::new(host.max.x, (y0 + w).min(host.max.y)),
		}
	} else {
		let cx = 0.5 * (host.min.x + host.max.x);
		let x0 = (cx - half).clamp(host.min.x, host.max.x - w);
		Aabb2d {
			min: Vec2::new(x0, host.min.y),
			max: Vec2::new((x0 + w).min(host.max.x), host.max.y),
		}
	}
}

fn band_from_port(host: Aabb2d, port: Vec2, w: f32) -> Aabb2d {
	let half = w * 0.5;
	let dist_w = (port.x - host.min.x).abs();
	let dist_e = (port.x - host.max.x).abs();
	let dist_s = (port.y - host.min.y).abs();
	let dist_n = (port.y - host.max.y).abs();
	let edge = [dist_w, dist_e, dist_s, dist_n]
		.iter()
		.enumerate()
		.min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
		.map(|(i, _)| i)
		.unwrap_or(0);
	match edge {
		0 | 1 => {
			// Vertical face → horizontal band from that edge through center.
			let y0 = (port.y - half).clamp(host.min.y, host.max.y - w);
			Aabb2d {
				min: Vec2::new(host.min.x, y0),
				max: Vec2::new(host.max.x, y0 + w),
			}
		}
		_ => {
			let x0 = (port.x - half).clamp(host.min.x, host.max.x - w);
			Aabb2d {
				min: Vec2::new(x0, host.min.y),
				max: Vec2::new(x0 + w, host.max.y),
			}
		}
	}
}

fn passage_lip_band(confines: &Confines, min_hall: f32) -> Option<Aabb2d> {
	let host = host_xz(&confines.bounds);
	let ports = passage_port_points(confines, &host);
	let port = *ports.first()?;
	let lip = min_hall.min((host.max - host.min).min_element() * 0.35);
	let dist_w = (port.x - host.min.x).abs();
	let dist_e = (port.x - host.max.x).abs();
	let dist_s = (port.y - host.min.y).abs();
	let dist_n = (port.y - host.max.y).abs();
	let edge = [dist_w, dist_e, dist_s, dist_n]
		.iter()
		.enumerate()
		.min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
		.map(|(i, _)| i)?;
	let door_w = DOOR_WIDTH.max(min_hall);
	let half = door_w * 0.5;
	Some(match edge {
		0 => Aabb2d {
			min: Vec2::new(host.min.x, (port.y - half).clamp(host.min.y, host.max.y - door_w)),
			max: Vec2::new(
				(host.min.x + lip).min(host.max.x),
				(port.y - half).clamp(host.min.y, host.max.y - door_w) + door_w,
			),
		},
		1 => Aabb2d {
			min: Vec2::new(
				(host.max.x - lip).max(host.min.x),
				(port.y - half).clamp(host.min.y, host.max.y - door_w),
			),
			max: Vec2::new(
				host.max.x,
				(port.y - half).clamp(host.min.y, host.max.y - door_w) + door_w,
			),
		},
		2 => Aabb2d {
			min: Vec2::new((port.x - half).clamp(host.min.x, host.max.x - door_w), host.min.y),
			max: Vec2::new(
				(port.x - half).clamp(host.min.x, host.max.x - door_w) + door_w,
				(host.min.y + lip).min(host.max.y),
			),
		},
		_ => Aabb2d {
			min: Vec2::new(
				(port.x - half).clamp(host.min.x, host.max.x - door_w),
				(host.max.y - lip).max(host.min.y),
			),
			max: Vec2::new(
				(port.x - half).clamp(host.min.x, host.max.x - door_w) + door_w,
				host.max.y,
			),
		},
	})
}

fn pack_closed_abutting(
	free: &mut Vec<Aabb2d>,
	kinds: &[RectQuarterKind],
	spine: &[Aabb2d],
	min_hall: f32,
	y0: f32,
	y1: f32,
	roll: f32,
	noise: NoiseParams,
	rooms: &mut Vec<RectAreaRoom>,
	closed_confines: &mut Vec<Confines>,
	filled_closed: &mut Vec<Confines>,
	residual_within: &mut Vec<FillRegion>,
) -> Result<(), FitError> {
	for &kind in kinds {
		let Some(idx) = pick_largest_abutting_host(free, spine, min_hall, min_area_for(kind)) else {
			continue;
		};
		let host = free.remove(idx);
		let cell_noise = noise_for_cell(noise, rooms.len() as i32);
		let (slot, rem) = take_slot(host, kind);
		let try_slot = |slot: Aabb2d| {
			let openings = door_onto_spine(slot, spine, y0, y1, rooms.len() as u32);
			let confines = confines_from_xz(slot, y0, y1, roll, &openings);
			try_fit_kind(kind, &confines, cell_noise).map(|(room, nested)| (room, nested, confines))
		};
		match try_slot(slot) {
			Ok((room, nested, confines)) => {
				rooms.push(room);
				closed_confines.push(confines.clone());
				filled_closed.push(confines);
				residual_within.extend(nested.within);
				for r in rem {
					if rect_usable(r, 3.0) {
						free.push(r);
					} else if aabb2_area(r) > EPS {
						let scrap = confines_from_xz(r, y0, y1, roll, &Openings::new());
						push_leftover(rooms, residual_within, scrap);
					}
				}
			}
			Err(FitError::TooSmall { .. }) if !aabb2_near_eq(slot, host) => {
				match try_slot(host) {
					Ok((room, nested, confines)) => {
						rooms.push(room);
						closed_confines.push(confines.clone());
						filled_closed.push(confines);
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

fn pack_open_into(
	free: &mut Vec<Aabb2d>,
	kinds: &[RectQuarterKind],
	spine: Option<&[Aabb2d]>,
	y0: f32,
	y1: f32,
	roll: f32,
	noise: NoiseParams,
	rooms: &mut Vec<RectAreaRoom>,
	open_confines: &mut Vec<Confines>,
	residual_within: &mut Vec<FillRegion>,
) -> Result<(), FitError> {
	for &kind in kinds {
		try_pack_one_open(
			free,
			kind,
			spine,
			y0,
			y1,
			roll,
			noise,
			rooms,
			open_confines,
			residual_within,
		)?;
	}
	// After the program's open kinds (incl. at most one Eating) are placed,
	// leftover pockets become living/sitting — not more kitchens.
	let fillers = [RectQuarterKind::Living, RectQuarterKind::Sitting];
	let mut guard = 0;
	while guard < 12 {
		guard += 1;
		let mut placed = false;
		for &kind in &fillers {
			let before = rooms.len();
			try_pack_one_open(
				free,
				kind,
				spine,
				y0,
				y1,
				roll,
				noise,
				rooms,
				open_confines,
				residual_within,
			)?;
			if rooms.len() > before {
				placed = true;
				break;
			}
		}
		if !placed {
			break;
		}
	}
	Ok(())
}

fn try_pack_one_open(
	free: &mut Vec<Aabb2d>,
	kind: RectQuarterKind,
	spine: Option<&[Aabb2d]>,
	y0: f32,
	y1: f32,
	roll: f32,
	noise: NoiseParams,
	rooms: &mut Vec<RectAreaRoom>,
	open_confines: &mut Vec<Confines>,
	residual_within: &mut Vec<FillRegion>,
) -> Result<(), FitError> {
	let Some(idx) = pick_largest_host(free, min_area_for(kind)) else {
		return Ok(());
	};
	let host = free.remove(idx);
	let cell_noise = noise_for_cell(noise, rooms.len() as i32);
	let (slot, rem) = take_slot(host, kind);
	let try_slot = |slot: Aabb2d| {
		// Quarters require a passage for placer keep-outs (`reason: "passage"`).
		let openings = match spine {
			Some(bands) => door_onto_spine(slot, bands, y0, y1, rooms.len() as u32),
			None => slot_edge_passage(slot, y0, y1, rooms.len() as u32),
		};
		let confines = confines_from_xz(slot, y0, y1, roll, &openings);
		try_fit_kind(kind, &confines, cell_noise).map(|(room, nested)| (room, nested, confines))
	};
	match try_slot(slot) {
		Ok((room, nested, confines)) => {
			rooms.push(room);
			open_confines.push(confines);
			residual_within.extend(nested.within);
			return_open_remnants(free, rooms, residual_within, rem, y0, y1, roll);
		}
		Err(FitError::TooSmall { .. }) if !aabb2_near_eq(slot, host) => match try_slot(host) {
			Ok((room, nested, confines)) => {
				rooms.push(room);
				open_confines.push(confines);
				residual_within.extend(nested.within);
			}
			Err(FitError::TooSmall { .. }) => free.push(host),
			Err(err) => return Err(err),
		},
		Err(FitError::TooSmall { .. }) => free.push(host),
		Err(err) => return Err(err),
	}
	Ok(())
}

fn pick_largest_host(free: &[Aabb2d], min_area: f32) -> Option<usize> {
	let mut best: Option<(usize, f32)> = None;
	for (i, r) in free.iter().enumerate() {
		if !rect_usable(*r, min_area) {
			continue;
		}
		let a = aabb2_area(*r);
		if best.map(|(_, ba)| a > ba).unwrap_or(true) {
			best = Some((i, a));
		}
	}
	best.map(|(i, _)| i)
}

fn return_open_remnants(
	free: &mut Vec<Aabb2d>,
	rooms: &mut Vec<RectAreaRoom>,
	residual_within: &mut Vec<FillRegion>,
	rem: Vec<Aabb2d>,
	y0: f32,
	y1: f32,
	roll: f32,
) {
	for r in rem {
		if rect_usable(r, 4.0) {
			free.push(r);
		} else if aabb2_area(r) > EPS {
			let scrap = confines_from_xz(r, y0, y1, roll, &Openings::new());
			push_leftover(rooms, residual_within, scrap);
		}
	}
}

fn door_onto_spine(slot: Aabb2d, spine: &[Aabb2d], y0: f32, y1: f32, id: u32) -> Openings {
	let mut openings = Openings::new();
	let mut best: Option<(bool, f32, f32, f32, f32)> = None;
	for s in spine {
		if let Some((along_x, lo, hi, mid)) = shared_edge_span(slot, *s) {
			let len = hi - lo;
			if best.map(|(_, _, _, _, l)| len > l).unwrap_or(true) {
				best = Some((along_x, lo, hi, mid, len));
			}
		}
	}
	if let Some((along_x, lo, hi, mid, _)) = best {
		if let Some((oid, opening)) = connecting_passage(along_x, lo, hi, mid, y0, y1, 0, id, 99) {
			openings.insert(oid, opening);
			return openings;
		}
	}
	// Slot may overlap the spine (open rooms) without a clean shared edge.
	slot_edge_passage(slot, y0, y1, id)
}

/// Synthetic door on the longest cardinal edge so quarter placers can clear entry.
fn slot_edge_passage(xz: Aabb2d, y0: f32, y1: f32, slot_id: u32) -> Openings {
	let mut openings = Openings::new();
	let sx = xz.max.x - xz.min.x;
	let sz = xz.max.y - xz.min.y;
	let door_w = DOOR_WIDTH.min(sx.max(sz) - 0.25).clamp(0.7, 1.15);
	let half = door_w * 0.5;
	let door_h = (y1 - y0).min(2.15).max(1.9);
	let half_d = 0.12_f32;
	let bounds = if sx >= sz {
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
		OpeningId::scoped(SCOPE, "open_door", format!("{slot_id}")),
		Opening::new(bounds, OpeningLabel::Passage),
	);
	openings
}

fn pick_largest_abutting_host(
	free: &[Aabb2d],
	spine: &[Aabb2d],
	min_hall: f32,
	min_area: f32,
) -> Option<usize> {
	let mut best: Option<(usize, f32)> = None;
	for (i, r) in free.iter().enumerate() {
		if !rect_usable(*r, min_area) {
			continue;
		}
		let abuts = spine.iter().any(|s| {
			shared_edge_span(*r, *s).is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_hall)
		});
		if !abuts {
			continue;
		}
		let a = aabb2_area(*r);
		if best.map(|(_, ba)| a > ba).unwrap_or(true) {
			best = Some((i, a));
		}
	}
	best.map(|(i, _)| i)
}

fn take_slot(host: Aabb2d, kind: RectQuarterKind) -> (Aabb2d, Vec<Aabb2d>) {
	let host_a = aabb2_area(host);
	let want = slot_target_area(kind, host_a);
	// Open rooms prefer generous claims so living is not starved into thin scraps;
	// closed rooms still carve a target pocket when the host is huge.
	// Eating carves a compact kitchen pocket and leaves remainder for living/sitting.
	let carve_threshold = match kind {
		RectQuarterKind::Eating | RectQuarterKind::Kitchen => 2.6,
		k if k.is_open() => 2.2,
		_ => 1.7,
	};
	if host_a < want * carve_threshold {
		return (host, Vec::new());
	}
	let frac = match kind {
		RectQuarterKind::Eating => (want / host_a).clamp(0.22, 0.48),
		RectQuarterKind::Kitchen => (want / host_a).clamp(0.22, 0.45),
		k if k.is_open() => (want / host_a).clamp(0.38, 0.75),
		_ => (want / host_a).clamp(0.28, 0.65),
	};
	let min_d = min_dim_for(kind);
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

/// Aspirational slot area (m²). Living takes a generous share; eating stays
/// compact so one kitchen does not crowd out the living program.
fn slot_target_area(kind: RectQuarterKind, host_a: f32) -> f32 {
	match kind {
		RectQuarterKind::Bedroom => 18.0,
		RectQuarterKind::Living => (host_a * 0.42).clamp(12.0, 42.0),
		RectQuarterKind::Eating => (host_a * 0.24).clamp(6.0, 22.0),
		RectQuarterKind::Kitchen => (host_a * 0.2).clamp(5.0, 16.0),
		RectQuarterKind::Dining => (host_a * 0.18).clamp(5.0, 14.0),
		RectQuarterKind::Bathroom => 6.5,
		RectQuarterKind::HalfBath => 3.5,
		RectQuarterKind::Sitting => (host_a * 0.28).clamp(6.0, 24.0),
		RectQuarterKind::Study => 9.0,
	}
}

fn min_area_for(kind: RectQuarterKind) -> f32 {
	match kind {
		RectQuarterKind::Bedroom => 12.0,
		RectQuarterKind::Living => 9.0,
		RectQuarterKind::Eating => 5.0,
		RectQuarterKind::Kitchen => 4.8,
		RectQuarterKind::Dining => 4.4,
		RectQuarterKind::Bathroom => 4.5,
		RectQuarterKind::HalfBath => 2.0,
		RectQuarterKind::Sitting => 5.0,
		RectQuarterKind::Study => 5.0,
	}
}

fn min_dim_for(kind: RectQuarterKind) -> f32 {
	match kind {
		RectQuarterKind::Bedroom => 3.2,
		RectQuarterKind::Bathroom | RectQuarterKind::HalfBath => 1.6,
		_ => MIN_ROOM,
	}
}

fn rect_usable(r: Aabb2d, min_area: f32) -> bool {
	let s = r.max - r.min;
	s.x + EPS >= MIN_ROOM && s.y + EPS >= MIN_ROOM && aabb2_area(r) + EPS >= min_area
}

fn aabb2_near_eq(a: Aabb2d, b: Aabb2d) -> bool {
	(a.min.x - b.min.x).abs() < 0.05
		&& (a.min.y - b.min.y).abs() < 0.05
		&& (a.max.x - b.max.x).abs() < 0.05
		&& (a.max.y - b.max.y).abs() < 0.05
}

fn try_fit_kind(
	kind: RectQuarterKind,
	confines: &Confines,
	noise: NoiseParams,
) -> Result<(RectAreaRoom, FillableRegions), FitError> {
	let fallbacks: &[RectQuarterKind] = match kind {
		RectQuarterKind::Bedroom => &[
			RectQuarterKind::Bedroom,
			RectQuarterKind::Study,
			RectQuarterKind::Sitting,
		],
		RectQuarterKind::Living => &[
			RectQuarterKind::Living,
			RectQuarterKind::Sitting,
			RectQuarterKind::Eating,
		],
		RectQuarterKind::Eating => &[RectQuarterKind::Eating, RectQuarterKind::Kitchen],
		RectQuarterKind::Kitchen => &[
			RectQuarterKind::Eating,
			RectQuarterKind::Kitchen,
			RectQuarterKind::Dining,
		],
		RectQuarterKind::Dining => &[
			RectQuarterKind::Eating,
			RectQuarterKind::Dining,
			RectQuarterKind::Kitchen,
			RectQuarterKind::Living,
		],
		RectQuarterKind::Bathroom => &[RectQuarterKind::Bathroom, RectQuarterKind::HalfBath],
		RectQuarterKind::HalfBath => &[RectQuarterKind::HalfBath, RectQuarterKind::Bathroom],
		RectQuarterKind::Sitting => &[
			RectQuarterKind::Sitting,
			RectQuarterKind::Living,
			RectQuarterKind::Study,
		],
		RectQuarterKind::Study => &[
			RectQuarterKind::Study,
			RectQuarterKind::Bedroom,
			RectQuarterKind::Sitting,
		],
	};
	let mut last = FitError::TooSmall {
		reason: "rla_quarter",
	};
	for &k in fallbacks {
		match fit_kind_exact(k, confines, noise) {
			Ok(ok) => return Ok(ok),
			Err(FitError::TooSmall { reason }) => last = FitError::TooSmall { reason },
			Err(err) => return Err(err),
		}
	}
	Err(last)
}

fn fit_kind_exact(
	kind: RectQuarterKind,
	confines: &Confines,
	noise: NoiseParams,
) -> Result<(RectAreaRoom, FillableRegions), FitError> {
	match kind {
		RectQuarterKind::Bedroom => CommonBedroom::fit_to_confines(confines, noise)
			.map(|(r, n)| (RectAreaRoom::Bedroom(r), n)),
		RectQuarterKind::Living => LivingRoom::fit_to_confines(confines, noise)
			.map(|(r, n)| (RectAreaRoom::Living(r), n)),
		RectQuarterKind::Eating => EatingArea::fit_to_confines(confines, noise)
			.map(|(r, n)| (RectAreaRoom::Eating(r), n)),
		RectQuarterKind::Kitchen => Kitchen::fit_to_confines(confines, noise)
			.map(|(r, n)| (RectAreaRoom::Kitchen(r), n)),
		RectQuarterKind::Dining => DiningRoom::fit_to_confines(confines, noise)
			.map(|(r, n)| (RectAreaRoom::Dining(r), n)),
		RectQuarterKind::Bathroom => ResidentialBathroom::fit_to_confines(confines, noise)
			.map(|(r, n)| (RectAreaRoom::Bathroom(r), n)),
		RectQuarterKind::HalfBath => ResidentialHalfBathroom::fit_to_confines(confines, noise)
			.map(|(r, n)| (RectAreaRoom::HalfBath(r), n)),
		RectQuarterKind::Sitting => SittingRoom::fit_to_confines(confines, noise)
			.map(|(r, n)| (RectAreaRoom::Sitting(r), n)),
		RectQuarterKind::Study => {
			Study::fit_to_confines(confines, noise).map(|(r, n)| (RectAreaRoom::Study(r), n))
		}
	}
}

fn push_leftover(rooms: &mut Vec<RectAreaRoom>, residual: &mut Vec<FillRegion>, confines: Confines) {
	let area = {
		let fp = confines.footprint();
		fp.x * fp.y
	};
	if (1.8..8.0).contains(&area) {
		rooms.push(RectAreaRoom::HouseholdCloset {
			label: label_filling_aabb(
				LabelStyle::Gray,
				"HouseholdCloset",
				&confines.bounds,
				confines.roll,
			),
			confines,
		});
	} else if area >= 8.0 {
		// Keep large unfilled pockets visible as open plan (not silent residual).
		rooms.push(RectAreaRoom::OpenBand {
			label: label_filling_aabb(
				LabelStyle::Cyan,
				"OpenPlan",
				&confines.bounds,
				confines.roll,
			),
			confines: confines.clone(),
		});
		residual.push(FillRegion::new(SpaceKind::InternalSpace, confines));
	} else if area > EPS {
		residual.push(FillRegion::new(SpaceKind::InternalSpace, confines));
	}
}

fn noise_for_cell(noise: NoiseParams, cell: i32) -> NoiseParams {
	NoiseParams {
		seed: noise.seed.wrapping_add(cell.wrapping_mul(97)),
		..noise
	}
}

fn split_program(program: &[RectQuarterKind]) -> (Vec<RectQuarterKind>, Vec<RectQuarterKind>) {
	let mut a = Vec::new();
	let mut b = Vec::new();
	for (i, &k) in program.iter().enumerate() {
		if i % 2 == 0 {
			a.push(k);
		} else {
			b.push(k);
		}
	}
	if a.is_empty() {
		a.push(RectQuarterKind::Living);
	}
	if b.is_empty() {
		b.push(RectQuarterKind::Living);
	}
	(a, b)
}

fn split_host_openings(host: &Confines, a: Aabb2d, b: Aabb2d) -> (Openings, Openings) {
	let mut oa = Openings::new();
	let mut ob = Openings::new();
	for (id, o) in host.openings.iter() {
		if !matches!(o.label, OpeningLabel::Passage) {
			continue;
		}
		let dmin = Vec3::from(o.bounds.min);
		let dmax = Vec3::from(o.bounds.max);
		let c = Vec2::new(0.5 * (dmin.x + dmax.x), 0.5 * (dmin.z + dmax.z));
		let da = (c - a.center()).length_squared();
		let db = (c - b.center()).length_squared();
		if da <= db {
			oa.insert(id.clone(), o.clone());
		} else {
			ob.insert(id.clone(), o.clone());
		}
	}
	(oa, ob)
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

fn connecting_passage(
	along_x: bool,
	lo: f32,
	hi: f32,
	mid: f32,
	y0: f32,
	y1: f32,
	area_id: u32,
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
		OpeningId::scoped(SCOPE, "connect", format!("{area_id}_{a}_{b}")),
		Opening::new(bounds, OpeningLabel::Passage),
	))
}

fn enclose_closed_rooms(
	closed: &[Confines],
	open_bands: &[Aabb2d],
	area_id: u32,
) -> Vec<ClippedRectangularStrip> {
	let thickness = DEFAULT_PANEL_THICKNESS.max(0.12);
	let mut partitions = Vec::new();
	for (i, c) in closed.iter().enumerate() {
		let y0 = Vec3::from(c.bounds.min).y;
		let y1 = Vec3::from(c.bounds.max).y;
		let b = host_xz(&c.bounds);
		let faces = [
			(false, b.min.x, b.min.y, b.max.y),
			(false, b.max.x, b.min.y, b.max.y),
			(true, b.min.y, b.min.x, b.max.x),
			(true, b.max.y, b.min.x, b.max.x),
		];
		let mut door_placed = false;
		let mut best_door: Option<usize> = None;
		let mut best_score = f32::NEG_INFINITY;
		for (fi, &(along_x, mid, lo, hi)) in faces.iter().enumerate() {
			let hall_len = open_bands
				.iter()
				.filter_map(|hall| shared_edge_span(b, *hall))
				.filter(|(ax, flo, fhi, fmid)| {
					*ax == along_x
						&& (*fmid - mid).abs() < 0.08
						&& *flo <= hi + EPS
						&& *fhi >= lo - EPS
				})
				.map(|(_, flo, fhi, _)| fhi - flo)
				.fold(0.0_f32, f32::max);
			if hall_len > best_score {
				best_score = hall_len;
				best_door = Some(fi);
			}
		}
		for (fi, &(along_x, mid, lo, hi)) in faces.iter().enumerate() {
			if hi - lo < EPS {
				continue;
			}
			let door = if !door_placed && best_door == Some(fi) && best_score >= DOOR_WIDTH * 0.7
			{
				door_placed = true;
				connecting_passage(
					along_x,
					lo,
					hi,
					mid,
					y0,
					y1,
					area_id,
					i as u32,
					9000 + fi as u32,
				)
			} else {
				None
			};
			push_private_wall(
				&mut partitions,
				along_x,
				lo,
				hi,
				mid,
				y0,
				y1,
				thickness,
				door,
			);
		}
	}
	partitions
}

fn push_private_wall(
	partitions: &mut Vec<ClippedRectangularStrip>,
	along_x: bool,
	lo: f32,
	hi: f32,
	mid: f32,
	y0: f32,
	y1: f32,
	thickness: f32,
	door: Option<(OpeningId, Opening)>,
) {
	let height = (y1 - y0).max(2.0);
	let openings = match door {
		Some((id, opening)) => {
			let mut o = Openings::new();
			o.insert(id, opening);
			o
		}
		None => Openings::new(),
	};
	if let Some(wall) = partition_strip(along_x, lo, hi, mid, y0, height, thickness, &openings) {
		partitions.push(wall);
	}
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
	if hi - lo < EPS {
		return None;
	}
	let (start, end, outward) = if along_x {
		(
			Vec3::new(lo, y0, mid),
			Vec3::new(hi, y0, mid),
			Vec2::new(0.0, 1.0),
		)
	} else {
		(
			Vec3::new(mid, y0, lo),
			Vec3::new(mid, y0, hi),
			Vec2::new(1.0, 0.0),
		)
	};
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

/// Authored passage openings for playground cells (on specified host faces).
pub fn passages_on_faces(
	host: Aabb2d,
	y0: f32,
	y1: f32,
	faces: &[(CardinalFace, f32)],
) -> Openings {
	let mut openings = Openings::new();
	for (i, &(face, t)) in faces.iter().enumerate() {
		let door_w = DOOR_WIDTH;
		let half = door_w * 0.5;
		let door_h = (y1 - y0).min(2.15).max(1.9);
		let half_d = 0.12_f32;
		let bounds = match face {
			CardinalFace::West => {
				let y = host.min.y + (host.max.y - host.min.y) * t;
				Aabb3d::from_min_max(
					Vec3::new(host.min.x - half_d, y0, y - half),
					Vec3::new(host.min.x + half_d, y0 + door_h, y + half),
				)
			}
			CardinalFace::East => {
				let y = host.min.y + (host.max.y - host.min.y) * t;
				Aabb3d::from_min_max(
					Vec3::new(host.max.x - half_d, y0, y - half),
					Vec3::new(host.max.x + half_d, y0 + door_h, y + half),
				)
			}
			CardinalFace::South => {
				let x = host.min.x + (host.max.x - host.min.x) * t;
				Aabb3d::from_min_max(
					Vec3::new(x - half, y0, host.min.y - half_d),
					Vec3::new(x + half, y0 + door_h, host.min.y + half_d),
				)
			}
			CardinalFace::North => {
				let x = host.min.x + (host.max.x - host.min.x) * t;
				Aabb3d::from_min_max(
					Vec3::new(x - half, y0, host.max.y - half_d),
					Vec3::new(x + half, y0 + door_h, host.max.y + half_d),
				)
			}
		};
		openings.insert(
			OpeningId::scoped(SCOPE, "port", format!("{i}")),
			Opening::new(bounds, OpeningLabel::Passage),
		);
	}
	openings
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalFace {
	West,
	East,
	South,
	North,
}

#[cfg(test)]
mod tests {
	use super::*;

	fn rect_with_south_door(sx: f32, sz: f32) -> Confines {
		let host = Aabb2d {
			min: Vec2::ZERO,
			max: Vec2::new(sx, sz),
		};
		let openings = passages_on_faces(host, 0.0, 3.0, &[(CardinalFace::South, 0.5)]);
		confines_from_xz(host, 0.0, 3.0, 0.0, &openings)
	}

	#[test]
	fn single_passage_small_may_close() {
		let confines = rect_with_south_door(4.0, 5.0);
		let params = RectangularLivableAreaParameterized {
			strategy: RectLivableStrategy::SingleClosed,
			..Default::default()
		};
		let (area, _) = RectangularLivableArea::fit_with_params(
			&confines,
			NoiseParams::default(),
			params,
			&[RectQuarterKind::Bedroom],
		)
		.unwrap();
		assert!(
			area.rooms
				.iter()
				.any(|r| matches!(r, RectAreaRoom::Bedroom(_))),
			"expected closed bedroom"
		);
		assert!(normalize_ok(&area, 1.0));
	}

	#[test]
	fn multi_passage_open_connects() {
		let host = Aabb2d {
			min: Vec2::ZERO,
			max: Vec2::new(8.0, 6.0),
		};
		let openings = passages_on_faces(
			host,
			0.0,
			3.0,
			&[
				(CardinalFace::South, 0.5),
				(CardinalFace::North, 0.5),
			],
		);
		let confines = confines_from_xz(host, 0.0, 3.0, 0.0, &openings);
		let params = RectangularLivableAreaParameterized {
			strategy: RectLivableStrategy::CaseAttempt,
			..Default::default()
		};
		let (area, _) = RectangularLivableArea::fit_with_params(
			&confines,
			NoiseParams { seed: 2, ..Default::default() },
			params,
			&[RectQuarterKind::Eating, RectQuarterKind::Living],
		)
		.unwrap();
		assert!(normalize_ok(&area, 1.0));
		assert!(passage_count(&area.confines) >= 2);
	}

	#[test]
	fn large_spine_packs_open_quarters() {
		let host = Aabb2d {
			min: Vec2::ZERO,
			max: Vec2::new(12.0, 14.0),
		};
		let openings = passages_on_faces(host, 0.0, 3.2, &[(CardinalFace::South, 0.5)]);
		let confines = confines_from_xz(host, 0.0, 3.2, 0.0, &openings);
		let params = RectangularLivableAreaParameterized {
			strategy: RectLivableStrategy::SpineHall,
			..Default::default()
		};
		let (area, _) = RectangularLivableArea::fit_with_params(
			&confines,
			NoiseParams {
				seed: 7,
				..Default::default()
			},
			params,
			&[
				RectQuarterKind::Eating,
				RectQuarterKind::Living,
				RectQuarterKind::Bedroom,
				RectQuarterKind::Bathroom,
			],
		)
		.unwrap();
		assert!(
			area.rooms.iter().any(|r| matches!(
				r,
				RectAreaRoom::Eating(_) | RectAreaRoom::Kitchen(_)
			)),
			"expected eating area / kitchen in large spine layout"
		);
		assert!(
			area.rooms.iter().any(|r| match r {
				RectAreaRoom::Living(_) | RectAreaRoom::Sitting(_) => true,
				RectAreaRoom::Eating(e) => e.has_dining(),
				_ => false,
			}),
			"expected living/sitting or dining-in-eating"
		);
		let labeled: f32 = area
			.rooms
			.iter()
			.map(|r| match r {
				RectAreaRoom::OpenBand { confines, .. }
				| RectAreaRoom::HouseholdCloset { confines, .. } => {
					let f = confines.footprint();
					f.x * f.y
				}
				RectAreaRoom::Bedroom(x) => {
					x.room_type.placement.scale.x * x.room_type.placement.scale.z
				}
				RectAreaRoom::Study(x) => {
					x.room_type.placement.scale.x * x.room_type.placement.scale.z
				}
				RectAreaRoom::Living(x) => {
					x.room_type.placement.scale.x * x.room_type.placement.scale.z
				}
				RectAreaRoom::Sitting(x) => {
					x.room_type.placement.scale.x * x.room_type.placement.scale.z
				}
				RectAreaRoom::Eating(x) => {
					x.room_type.placement.scale.x * x.room_type.placement.scale.z
				}
				RectAreaRoom::Kitchen(x) => {
					x.room_type.placement.scale.x * x.room_type.placement.scale.z
				}
				RectAreaRoom::Dining(x) => {
					x.room_type.placement.scale.x * x.room_type.placement.scale.z
				}
				RectAreaRoom::Bathroom(x) => {
					x.room_type.placement.scale.x * x.room_type.placement.scale.z
				}
				RectAreaRoom::HalfBath(x) => {
					x.room_type.placement.scale.x * x.room_type.placement.scale.z
				}
			})
			.sum();
		// Most of the footprint should be claimed by labeled rooms / open bands.
		assert!(
			labeled > 12.0 * 14.0 * 0.55,
			"expected most of host labeled, got {labeled}"
		);
	}
}

