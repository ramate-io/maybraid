//! Corner keep: a circular or trazaloid shell plus storey-to-storey stairwells.

use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::partitions::PartitionStyle;
use richmond_building_components::{
	BuildingComponents, FloorNode, JointNode, Layers, PanelNode, PartitionNode,
};
use richmond_buildings::{
	ArcFloor, ArcFloorSlab, ArcTower, ArcTowerParams, ConnectingStairwell, Opening, OpeningId,
	OpeningLabel, Openings, StairwellKind, Trazaloid, TrazaloidParams, TrazaloidSide,
	TrazaloidSlab, WellAabb, WellSide,
};

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};

pub const TOWER_STOREY_HEIGHT: f32 = 3.2;
const KEEP_TREAD_FILL: f32 = 0.55;

/// Shell plus the wells that climb it.
#[derive(Debug, Clone, PartialEq)]
pub struct Keep<S> {
	pub shell: S,
	pub stairwells: Vec<ConnectingStairwell>,
}

impl<S> Keep<S> {
	pub fn new(shell: S, stairwells: Vec<ConnectingStairwell>) -> Self {
		Self { shell, stairwells }
	}
}

/// Circular keep: stacked [`ArcTower`] storeys with an optional wall shader stamp.
#[derive(Debug, Clone, PartialEq)]
pub struct CircularTower {
	tower: ArcTower,
	wall_material: Option<MaterialRef>,
}

impl CircularTower {
	pub fn inner(&self) -> &ArcTower {
		&self.tower
	}

	pub fn storey_count(&self) -> usize {
		self.tower.params().floor_count as usize
	}

	pub fn center_xz(&self) -> Vec3 {
		self.tower.params().center_xz
	}

	pub fn radius(&self) -> f32 {
		self.tower.params().radius
	}

	pub fn with_wall_material(mut self, wall: MaterialRef) -> Self {
		self.wall_material = Some(wall);
		self
	}
}

impl BuildingComponents for CircularTower {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		let mut out = self.tower.partition_nodes_for_level(level);
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		self.tower.floor_nodes_for_level(level)
	}
}

/// Vertically stacked trazaloid storeys sharing one tapering silhouette.
#[derive(Debug, Clone, PartialEq)]
pub struct TrazaloidTower {
	storeys: Vec<Trazaloid>,
	wall_material: Option<MaterialRef>,
}

impl TrazaloidTower {
	pub fn storeys(&self) -> &[Trazaloid] {
		&self.storeys
	}

	pub fn storey_count(&self) -> usize {
		self.storeys.len()
	}

	pub fn origin(&self) -> Vec3 {
		self.storeys.first().map(|s| s.params().origin).unwrap_or(Vec3::ZERO)
	}

	pub fn base_half_extent(&self) -> f32 {
		self.storeys
			.first()
			.map(|s| s.params().footprint.x.max(s.params().footprint.y) * 0.5)
			.unwrap_or(0.0)
	}

	pub fn with_wall_material(mut self, wall: MaterialRef) -> Self {
		self.wall_material = Some(wall);
		self
	}
}

impl BuildingComponents for TrazaloidTower {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for storey in &self.storeys {
			out.extend(storey.panel_nodes_for_level(level));
		}
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for storey in &self.storeys {
			out.extend(storey.joint_nodes_for_level(level));
		}
		out
	}
}

/// Circular [`ArcTower`] or stacked [`Trazaloid`] keep at a ring-fort corner.
#[derive(Debug, Clone, PartialEq)]
pub enum RingFortKeep {
	Circular(Keep<CircularTower>),
	Trazaloid(Keep<TrazaloidTower>),
}

impl RingFortKeep {
	pub fn storey_count(&self) -> usize {
		match self {
			Self::Circular(keep) => keep.shell.storey_count(),
			Self::Trazaloid(keep) => keep.shell.storey_count(),
		}
	}

	pub fn center_xz(&self) -> Vec3 {
		match self {
			Self::Circular(keep) => keep.shell.center_xz(),
			Self::Trazaloid(keep) => keep.shell.origin(),
		}
	}

	pub fn plan_half_extent(&self) -> f32 {
		match self {
			Self::Circular(keep) => keep.shell.radius(),
			Self::Trazaloid(keep) => keep.shell.base_half_extent(),
		}
	}

	pub fn stairwells(&self) -> &[ConnectingStairwell] {
		match self {
			Self::Circular(keep) => &keep.stairwells,
			Self::Trazaloid(keep) => &keep.stairwells,
		}
	}

	pub fn with_wall_material(self, wall: MaterialRef) -> Self {
		match self {
			Self::Circular(keep) => Self::Circular(Keep {
				shell: keep.shell.with_wall_material(wall.clone()),
				stairwells: stamp_stairs(keep.stairwells, wall),
			}),
			Self::Trazaloid(keep) => Self::Trazaloid(Keep {
				shell: keep.shell.with_wall_material(wall.clone()),
				stairwells: stamp_stairs(keep.stairwells, wall),
			}),
		}
	}

	pub fn circular(origin: Vec3, radius: f32, floors: usize) -> Self {
		let well_half = keep_well_half(radius);
		let shell = build_circular_tower(origin, radius, floors, well_half * 2.0);
		let stairwells = keep_stairwells(origin, well_half, floors, StairwellKind::Circular);
		Self::Circular(Keep::new(shell, stairwells))
	}

	pub fn trazaloid(origin: Vec3, foot: f32, floors: usize, corner: (f32, f32)) -> Self {
		let n = floors.max(1) as f32;
		let t_top = (n - 1.0) / n;
		let top_foot = foot + (foot * 0.48 - foot) * t_top;
		let well_half = keep_well_half(top_foot * 0.5);
		let shell = build_trazaloid_tower(origin, foot, floors, corner, well_half);
		let stairwells = keep_stairwells(origin, well_half, floors, StairwellKind::Rectangular);
		Self::Trazaloid(Keep::new(shell, stairwells))
	}
}

impl BuildingComponents for RingFortKeep {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Circular(_) => Layers::new(),
			Self::Trazaloid(keep) => keep.shell.panel_nodes_for_level(level),
		}
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		match self {
			Self::Circular(keep) => keep.shell.partition_nodes_for_level(level),
			Self::Trazaloid(_) => Layers::new(),
		}
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		match self {
			Self::Circular(keep) => keep.shell.floor_nodes_for_level(level),
			Self::Trazaloid(_) => Layers::new(),
		}
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		match self {
			Self::Circular(_) => Layers::new(),
			Self::Trazaloid(keep) => keep.shell.joint_nodes_for_level(level),
		}
	}
}

fn stamp_stairs(
	stairwells: Vec<ConnectingStairwell>,
	wall: MaterialRef,
) -> Vec<ConnectingStairwell> {
	stairwells
		.into_iter()
		.map(|stair| stair.with_surface_material(wall.clone()))
		.collect()
}

fn keep_well_half(plan_half: f32) -> f32 {
	(plan_half * 0.32).clamp(1.7, 3.4)
}

fn keep_stairwells(
	origin: Vec3,
	well_half: f32,
	floors: usize,
	kind: StairwellKind,
) -> Vec<ConnectingStairwell> {
	if floors < 2 {
		return Vec::new();
	}
	let last_well_i = floors - 2;
	let mut out = Vec::with_capacity(floors - 1);
	for i in 0..=last_well_i {
		let y0 = origin.y + i as f32 * TOWER_STOREY_HEIGHT;
		let y1 = y0 + TOWER_STOREY_HEIGHT;
		let well = WellAabb::from_plan(
			Vec3::new(origin.x - well_half, y0, origin.z - well_half),
			Vec3::new(origin.x + well_half, y1, origin.z + well_half),
			WellSide::PosX,
			WellSide::NegX,
			KEEP_TREAD_FILL,
		);
		out.push(
			ConnectingStairwell::from_well_kind(PanelStyle::RoughStonework, well, kind)
				.with_upper_landing(i == last_well_i),
		);
	}
	out
}

fn shaft_opening(half: f32, y0: f32, y1: f32) -> Opening {
	Opening::new(
		Aabb3d::from_min_max(Vec3::new(-half, y0, -half), Vec3::new(half, y1, half)),
		OpeningLabel::Shaft,
	)
}

fn build_circular_tower(origin: Vec3, radius: f32, floors: usize, hole: f32) -> CircularTower {
	let mut openings = Openings::new();
	for (i, t) in [0.0_f32, 0.25, 0.5, 0.75].into_iter().enumerate() {
		let (id, opening) = ArcFloor::plan_opening_at_t(
			format!("window_{i}"),
			OpeningLabel::Aperture,
			origin,
			radius,
			TOWER_STOREY_HEIGHT,
			t,
		);
		openings.insert(id, opening);
	}
	CircularTower {
		tower: ArcTower::new(ArcTowerParams {
			center_xz: origin,
			radius,
			floor_count: floors as u32,
			storey_height: TOWER_STOREY_HEIGHT,
			openings,
			base_floor: ArcFloorSlab::Solid,
			intermediate_floors: ArcFloorSlab::Solid,
			top_ceiling: ArcFloorSlab::Solid,
			intermediate_floor_hole: hole,
			style: PartitionStyle::RoughStonework,
		}),
		wall_material: None,
	}
}

fn build_trazaloid_tower(
	origin: Vec3,
	foot: f32,
	floors: usize,
	corner: (f32, f32),
	well_half: f32,
) -> TrazaloidTower {
	let n = floors.max(1) as f32;
	let base = Vec2::splat(foot);
	let ridge_top = base * 0.48;
	let inner = inward_sides(corner.0, corner.1);
	let mut storeys = Vec::with_capacity(floors);
	for i in 0..floors {
		let t0 = i as f32 / n;
		let t1 = (i + 1) as f32 / n;
		let footprint = base.lerp(ridge_top, t0);
		let ridge = base.lerp(ridge_top, t1);
		let y = origin.y + i as f32 * TOWER_STOREY_HEIGHT;
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("stair_shaft"),
			shaft_opening(well_half, -0.25, TOWER_STOREY_HEIGHT + 0.25),
		);
		if i == 0 {
			for (k, side) in inner.into_iter().enumerate() {
				openings.insert(
					OpeningId::new(format!("gate_{k}")),
					Trazaloid::side_passage_opening(
						side,
						footprint,
						(footprint.x.min(footprint.y) * 0.22).clamp(1.1, 1.8),
						2.1,
					),
				);
			}
		}
		storeys.push(
			TrazaloidParams {
				origin: Vec3::new(origin.x, y, origin.z),
				footprint,
				ridge,
				lower_height: TOWER_STOREY_HEIGHT * 0.52,
				upper_height: TOWER_STOREY_HEIGHT * 0.35,
				band_vertical_offset: TOWER_STOREY_HEIGHT * 0.13,
				openings,
				floor: TrazaloidSlab::Solid,
				ceiling: if i + 1 == floors { TrazaloidSlab::Solid } else { TrazaloidSlab::None },
				style: PanelStyle::RoughStonework,
				..TrazaloidParams::default()
			}
			.build(),
		);
	}
	TrazaloidTower { storeys, wall_material: None }
}

fn inward_sides(sx: f32, sz: f32) -> [TrazaloidSide; 2] {
	let x_side = if sx > 0.0 { TrazaloidSide::West } else { TrazaloidSide::East };
	let z_side = if sz > 0.0 { TrazaloidSide::South } else { TrazaloidSide::North };
	[x_side, z_side]
}
