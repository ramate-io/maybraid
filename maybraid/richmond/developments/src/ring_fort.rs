//! Ring fort: a Les Halles courtyard ring with circular or trazaloid corner towers.
//!
//! The ring is 2–4 storeys (confines height). Each corner tower is 5–10 storeys.
//! Topology is a star: the ring is the hub, and each tower joins it at a corner.

use std::sync::Arc;

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::partitions::PartitionStyle;
use richmond_building_components::{
	BuildingComponents, FloorNode, JointNode, Layers, PanelNode, PartitionNode,
};
use richmond_buildings::{
	ArcFloor, ArcFloorSlab, ArcTower, ArcTowerParams, Confines, FillableRegions, Fit, FitError,
	MixedUseLesHallesStorey, OpeningId, OpeningLabel, Openings, Trazaloid, TrazaloidParams,
	TrazaloidSide, TrazaloidSlab,
};

use crate::connected::{ConnectedDevelopment, DevelopmentEdge};
use crate::les_halles::{MixedUseLesHallesDevelopment, MixedUseLesHallesHost};
use crate::placed::BuildingFootprint;

/// Minimum Les Halles ring plan so gallery + courtyard still fit.
const MIN_RING_PLAN: f32 = 36.0;
const TOWER_STOREY_MIN: usize = 5;
const TOWER_STOREY_MAX: usize = 10;
const CIRCULAR_RADIUS_MIN: f32 = 6.0;
const CIRCULAR_RADIUS_MAX: f32 = 9.0;
const TRAZALOID_FOOT_MIN: f32 = 10.0;
const TRAZALOID_FOOT_MAX: f32 = 14.0;
const TOWER_STOREY_HEIGHT: f32 = 3.2;

const SALT_KIND: f32 = 53.0;
const SALT_FLOORS: f32 = 59.0;
const SALT_SIZE: f32 = 61.0;

const CORNERS: [(f32, f32); 4] = [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)];

/// Corner join from a tower onto the courtyard ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RingFortJoin;

/// One site in a [`RingFort`]: the courtyard ring or a corner tower.
#[derive(Debug, Clone, PartialEq)]
pub enum RingFortSite {
	Ring(Box<MixedUseLesHallesDevelopment>),
	Tower(RingFortTower),
}

/// Circular [`ArcTower`] or stacked [`Trazaloid`] keep at a ring-fort corner.
#[derive(Debug, Clone, PartialEq)]
pub enum RingFortTower {
	Circular(ArcTower),
	Trazaloid(TrazaloidTower),
}

impl RingFortTower {
	pub fn storey_count(&self) -> usize {
		match self {
			Self::Circular(tower) => tower.params().floor_count as usize,
			Self::Trazaloid(tower) => tower.storey_count(),
		}
	}

	pub fn center_xz(&self) -> Vec3 {
		match self {
			Self::Circular(tower) => tower.params().center_xz,
			Self::Trazaloid(tower) => tower.origin(),
		}
	}

	pub fn plan_half_extent(&self) -> f32 {
		match self {
			Self::Circular(tower) => tower.params().radius,
			Self::Trazaloid(tower) => tower.base_half_extent(),
		}
	}
}

impl BuildingComponents for RingFortTower {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Circular(_) => Layers::new(),
			Self::Trazaloid(tower) => tower.panel_nodes_for_level(level),
		}
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		match self {
			Self::Circular(tower) => tower.partition_nodes_for_level(level),
			Self::Trazaloid(_) => Layers::new(),
		}
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		match self {
			Self::Circular(tower) => tower.floor_nodes_for_level(level),
			Self::Trazaloid(_) => Layers::new(),
		}
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		match self {
			Self::Circular(_) => Layers::new(),
			Self::Trazaloid(tower) => tower.joint_nodes_for_level(level),
		}
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

/// Sibling LOD host emitted by [`RingFort`].
#[derive(Debug, Clone, PartialEq)]
pub enum RingFortHost {
	Ring(Box<MixedUseLesHallesHost>),
	Circular(Arc<ArcTower>),
	Trazaloid(Arc<TrazaloidTower>),
}

/// Courtyard ring with four corner towers.
#[derive(Debug, Clone, PartialEq)]
pub struct RingFort {
	pub connected: ConnectedDevelopment<RingFortSite, RingFortJoin>,
}

impl RingFort {
	pub fn ring(&self) -> Option<&MixedUseLesHallesDevelopment> {
		self.connected.nodes.iter().find_map(|site| match site {
			RingFortSite::Ring(ring) => Some(ring.as_ref()),
			RingFortSite::Tower(_) => None,
		})
	}

	pub fn towers(&self) -> impl Iterator<Item = &RingFortTower> {
		self.connected.nodes.iter().filter_map(|site| match site {
			RingFortSite::Tower(tower) => Some(tower),
			RingFortSite::Ring(_) => None,
		})
	}

	pub fn hosts(&self) -> Vec<RingFortHost> {
		let mut out = Vec::new();
		for site in &self.connected.nodes {
			match site {
				RingFortSite::Ring(ring) => {
					out.extend(
						ring.hosts().into_iter().map(|host| RingFortHost::Ring(Box::new(host))),
					);
				}
				RingFortSite::Tower(RingFortTower::Circular(tower)) => {
					out.push(RingFortHost::Circular(Arc::new(tower.clone())));
				}
				RingFortSite::Tower(RingFortTower::Trazaloid(tower)) => {
					out.push(RingFortHost::Trazaloid(Arc::new(tower.clone())));
				}
			}
		}
		out
	}

	pub fn with_finish(mut self, wall: MaterialRef, roof: MaterialRef) -> Self {
		for site in &mut self.connected.nodes {
			match site {
				RingFortSite::Ring(ring) => {
					**ring = ring.as_ref().clone().with_finish(wall.clone(), roof.clone());
				}
				RingFortSite::Tower(RingFortTower::Trazaloid(tower)) => {
					*tower = tower.clone().with_wall_material(wall.clone());
				}
				RingFortSite::Tower(RingFortTower::Circular(_)) => {}
			}
		}
		self
	}
}

impl BuildingFootprint for RingFort {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		let mut rects = Vec::new();
		if let Some(ring) = self.ring() {
			rects.extend(ring.footprint_rects());
		}
		for tower in self.towers() {
			let c = tower.center_xz();
			let half = tower.plan_half_extent();
			rects.push(Aabb2d::new(Vec2::new(c.x, c.z), Vec2::splat(half)));
		}
		rects
	}
}

impl Fit for RingFort {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let extent = Vec3::from(confines.bounds.max - confines.bounds.min);
		let center = confines.center();
		let y0 = confines.bounds.min.y;
		let cfg = NoiseConfig::new(noise);
		let max_tower_half = ((extent.x.min(extent.z) - MIN_RING_PLAN) * 0.5).max(4.0);

		let mut specs = Vec::with_capacity(4);
		let mut max_half = 0.0_f32;
		for (i, _) in CORNERS.iter().enumerate() {
			let spec = sample_tower_spec(&cfg, center, i, max_tower_half);
			max_half = max_half.max(spec.half_extent());
			specs.push(spec);
		}

		let ring_w = (extent.x - 2.0 * max_half).max(0.0);
		let ring_d = (extent.z - 2.0 * max_half).max(0.0);
		if ring_w + 1e-3 < MIN_RING_PLAN || ring_d + 1e-3 < MIN_RING_PLAN {
			return Err(FitError::TooSmall { reason: "ring_fort_ring" });
		}

		let hx = ring_w * 0.5;
		let hz = ring_d * 0.5;
		let ring_confines = Confines::new(
			Aabb3d::from_min_max(
				Vec3::new(center.x - hx, y0, center.z - hz),
				Vec3::new(center.x + hx, confines.bounds.max.y, center.z + hz),
			),
			confines.roll,
			Openings::new(),
		);
		let (ring, _) = MixedUseLesHallesDevelopment::fit_to_confines(&ring_confines, noise)?;
		let plan = ring
			.tower
			.floors
			.first()
			.map(MixedUseLesHallesStorey::floor_plan)
			.ok_or(FitError::TooSmall { reason: "ring_fort_storeys" })?;
		let outer_h = plan.outer * 0.5;
		let ring_c = plan.center_xz;

		let mut nodes = Vec::with_capacity(5);
		nodes.push(RingFortSite::Ring(Box::new(ring)));
		let mut edges = Vec::with_capacity(4);
		for (i, &(sx, sz)) in CORNERS.iter().enumerate() {
			let tower_c = Vec3::new(ring_c.x + sx * outer_h.x, y0, ring_c.z + sz * outer_h.y);
			let tower = specs[i].build(tower_c, (sx, sz));
			let tower_i = nodes.len();
			nodes.push(RingFortSite::Tower(tower));
			edges.push(DevelopmentEdge::new(0, tower_i, RingFortJoin));
		}

		Ok((
			Self { connected: ConnectedDevelopment::new(confines.bounds, nodes, edges) },
			FillableRegions { within: Vec::new(), atop: Vec::new() },
		))
	}
}

struct TowerSpec {
	circular: bool,
	floors: usize,
	size: f32,
}

impl TowerSpec {
	fn half_extent(&self) -> f32 {
		if self.circular {
			self.size
		} else {
			self.size * 0.5
		}
	}

	fn build(&self, origin: Vec3, corner: (f32, f32)) -> RingFortTower {
		if self.circular {
			RingFortTower::Circular(build_circular_tower(origin, self.size, self.floors))
		} else {
			RingFortTower::Trazaloid(build_trazaloid_tower(origin, self.size, self.floors, corner))
		}
	}
}

fn sample_tower_spec(cfg: &NoiseConfig, center: Vec3, index: usize, max_half: f32) -> TowerSpec {
	let salt = index as f32;
	let circular = cfg.sample_unit_4d(center.x, center.y, center.z, SALT_KIND + salt) < 0.5;
	let floors = TOWER_STOREY_MIN
		+ cfg.sample_range_usize_4d(
			0,
			TOWER_STOREY_MAX - TOWER_STOREY_MIN + 1,
			center.x,
			center.y,
			center.z,
			SALT_FLOORS + salt,
		);
	if circular {
		let lo = CIRCULAR_RADIUS_MIN.min(max_half);
		let hi = CIRCULAR_RADIUS_MAX.min(max_half).max(lo);
		let radius =
			cfg.sample_range_f32_4d(lo, hi, center.x, center.y, center.z, SALT_SIZE + salt);
		TowerSpec { circular: true, floors, size: radius }
	} else {
		let max_foot = (max_half * 2.0).max(TRAZALOID_FOOT_MIN);
		let lo = TRAZALOID_FOOT_MIN.min(max_foot);
		let hi = TRAZALOID_FOOT_MAX.min(max_foot).max(lo);
		let foot = cfg.sample_range_f32_4d(lo, hi, center.x, center.y, center.z, SALT_SIZE + salt);
		TowerSpec { circular: false, floors, size: foot }
	}
}

fn build_circular_tower(origin: Vec3, radius: f32, floors: usize) -> ArcTower {
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
	ArcTower::new(ArcTowerParams {
		center_xz: origin,
		radius,
		floor_count: floors as u32,
		storey_height: TOWER_STOREY_HEIGHT,
		openings,
		base_floor: ArcFloorSlab::Solid,
		intermediate_floors: ArcFloorSlab::Solid,
		top_ceiling: ArcFloorSlab::Solid,
		intermediate_floor_hole: 2.24,
		style: PartitionStyle::RoughStonework,
	})
}

fn build_trazaloid_tower(
	origin: Vec3,
	foot: f32,
	floors: usize,
	corner: (f32, f32),
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

#[cfg(test)]
mod tests {
	use super::*;

	fn fort_bounds() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-40.0, 0.0, -40.0), Vec3::new(40.0, 12.0, 40.0))
	}

	fn fit_fort(seed: i32) -> anyhow::Result<RingFort> {
		let confines = Confines::from_bounds(fort_bounds());
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		RingFort::fit_to_confines(&confines, noise)
			.map(|(fort, _)| fort)
			.map_err(|err| anyhow::anyhow!("ring fort fit failed: {err:?}"))
	}

	#[test]
	fn ring_is_two_to_four_storeys_with_four_taller_towers() -> anyhow::Result<()> {
		let fort = fit_fort(42)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let ring_floors = ring.tower.floor_count();
		anyhow::ensure!((2..=4).contains(&ring_floors), "ring storeys {ring_floors} outside 2..=4");
		let towers: Vec<_> = fort.towers().collect();
		anyhow::ensure!(towers.len() == 4, "expected 4 corner towers, got {}", towers.len());
		for tower in &towers {
			let n = tower.storey_count();
			anyhow::ensure!((5..=10).contains(&n), "tower storeys {n} outside 5..=10");
			anyhow::ensure!(n > ring_floors, "tower should out-climb the ring");
		}
		anyhow::ensure!(fort.connected.edges.len() == 4);
		anyhow::ensure!(fort.connected.topology_is_valid());
		Ok(())
	}

	#[test]
	fn towers_sit_on_ring_corners() -> anyhow::Result<()> {
		let fort = fit_fort(11)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let plan = ring
			.tower
			.floors
			.first()
			.map(MixedUseLesHallesStorey::floor_plan)
			.ok_or_else(|| anyhow::anyhow!("ring has no storey"))?;
		let hx = plan.outer.x * 0.5;
		let hz = plan.outer.y * 0.5;
		for tower in fort.towers() {
			let c = tower.center_xz();
			let dx = (c.x - plan.center_xz.x).abs();
			let dz = (c.z - plan.center_xz.z).abs();
			anyhow::ensure!((dx - hx).abs() < 1e-3, "tower x {dx} vs ring half {hx}");
			anyhow::ensure!((dz - hz).abs() < 1e-3, "tower z {dz} vs ring half {hz}");
			anyhow::ensure!((c.y - plan.center_xz.y).abs() < 1e-3);
		}
		Ok(())
	}

	#[test]
	fn hosts_include_ring_and_each_tower() -> anyhow::Result<()> {
		let fort = fit_fort(7)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let hosts = fort.hosts();
		let ring_hosts = ring.hosts().len();
		let tower_hosts = fort.towers().count();
		anyhow::ensure!(
			hosts.len() == ring_hosts + tower_hosts,
			"hosts {} vs ring {} + towers {}",
			hosts.len(),
			ring_hosts,
			tower_hosts
		);
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::Ring(_))));
		anyhow::ensure!(hosts
			.iter()
			.any(|h| { matches!(h, RingFortHost::Circular(_) | RingFortHost::Trazaloid(_)) }));
		Ok(())
	}

	#[test]
	fn storey_counts_vary_across_seeds() -> anyhow::Result<()> {
		let mut ring_counts = std::collections::BTreeSet::new();
		let mut tower_counts = std::collections::BTreeSet::new();
		let mut saw_circular = false;
		let mut saw_trazaloid = false;
		for seed in 0..24 {
			let fort = fit_fort(seed)?;
			let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing ring"))?;
			ring_counts.insert(ring.tower.floor_count());
			for tower in fort.towers() {
				tower_counts.insert(tower.storey_count());
				match tower {
					RingFortTower::Circular(_) => saw_circular = true,
					RingFortTower::Trazaloid(_) => saw_trazaloid = true,
				}
			}
		}
		anyhow::ensure!(ring_counts.len() >= 2, "ring storeys should vary, got {ring_counts:?}");
		anyhow::ensure!(saw_circular, "expected at least one circular tower");
		anyhow::ensure!(saw_trazaloid, "expected at least one trazaloid tower");
		Ok(())
	}
}
