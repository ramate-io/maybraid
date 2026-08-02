//! One circular storey shell: wall sweeps + optional floor / ceiling slabs.
//!
//! Openings are resolved in two layers:
//! 1. **Wall sweeps** — 15° sectors (AABB-approximated); solid runs merge to 90°/180°.
//! 2. **Floor / ceiling** — slab-cutting openings that hit a Solid slab contribute a
//!    centered hole sized from the intersection scale (or remove the slab entirely).
//!
//! Floor / ceiling [`ArcFloorSlab`] values are only [`None`](ArcFloorSlab::None) /
//! [`Solid`](ArcFloorSlab::Solid). They are mainly for towering ownership; openings
//! still map whether or not a slab is present, and can override a Solid slab.
//!
//! # Kit sweep convention
//!
//! Rough-stone arc kits start at local \(−X\) and sweep through \(−Z\) (clockwise in
//! plan). Sector \(i\) is the kit placed at yaw \(i \cdot 15°\), covering that CW
//! 15° wedge — not the CCW wedge.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::floors::{Floor, FloorNode};
use richmond_building_components::partitions::{Partition, PartitionNode, PartitionStyle};
use richmond_building_components::{BuildingComponents, Layers, Placement};

use crate::openings::{
	MappedOpening, MappedOpeningQuad, MappedOpenings, MapsOpenings, Opening, OpeningId,
	OpeningLabel, Openings,
};
use crate::portals::SLICE_Y_FRAC;

/// Kit segment size (degrees).
const SEG_DEG: f32 = 15.0;
const SECTORS: u32 = 24; // 360 / 15
/// Inscribed-square half-extent as a fraction of outer radius → full side = 1.4·R.
const INSCRIBED_HALF_FRAC: f32 = 0.7;
/// Floor / ceiling slab Y scale.
const FLOOR_SLAB_Y_SCALE: f32 = 0.2;
const EPS: f32 = 1e-4;

/// Horizontal storey slab presentation for towering ownership.
///
/// Openings may still cut a Solid slab (Layer 2). Prefer openings for voids;
/// keep these variants simple so stack ownership stays obvious.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcFloorSlab {
	/// Omit the slab entirely (no floor/ceiling geometry).
	None,
	/// Squared floor fill (inscribed-square caps + solid inscribed square).
	/// Openings that intersect this slab may dig a centered hole or remove it.
	Solid,
}

impl Default for ArcFloorSlab {
	fn default() -> Self {
		Self::None
	}
}

/// Authored parameters for an [`ArcFloor`] shell.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcFloorParams {
	/// Storey plan center; `y` is the floor elevation.
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	/// Opening plan applied via Layer 1 (walls) and Layer 2 (slabs).
	pub openings: Openings,
	/// Towering ownership hint; openings may override a Solid slab.
	pub floor: ArcFloorSlab,
	/// Towering ownership hint; openings may override a Solid slab.
	pub ceiling: ArcFloorSlab,
	pub style: PartitionStyle,
}

impl Default for ArcFloorParams {
	fn default() -> Self {
		Self {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			openings: Openings::new(),
			floor: ArcFloorSlab::None,
			ceiling: ArcFloorSlab::None,
			style: PartitionStyle::RoughStonework,
		}
	}
}

/// Builder separating authorship from opening resolution.
#[derive(Debug, Clone)]
pub struct ArcFloorBuilder {
	params: ArcFloorParams,
}

impl ArcFloorBuilder {
	pub fn new(center_xz: Vec3, radius: f32, storey_height: f32) -> Self {
		Self {
			params: ArcFloorParams {
				center_xz,
				radius,
				storey_height,
				..ArcFloorParams::default()
			},
		}
	}

	pub fn floor(mut self, floor: ArcFloorSlab) -> Self {
		self.params.floor = floor;
		self
	}

	pub fn ceiling(mut self, ceiling: ArcFloorSlab) -> Self {
		self.params.ceiling = ceiling;
		self
	}

	pub fn style(mut self, style: PartitionStyle) -> Self {
		self.params.style = style;
		self
	}

	pub fn openings(mut self, openings: Openings) -> Self {
		self.params.openings = openings;
		self
	}

	pub fn build(self) -> ArcFloor {
		ArcFloor::from_params(self.params)
	}
}

/// Per-sector wall resolution after Layer 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectorWall {
	/// Full-height solid arc strip.
	Solid,
	/// Lower band omitted; lintel / slice kept.
	LintelOnly,
	/// Both vertical bands omitted.
	Empty,
}

/// One circular storey: wall partitions + optional floor / ceiling.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcFloor {
	params: ArcFloorParams,
	wall_partitions: Vec<PartitionNode>,
	floor_nodes: Vec<FloorNode>,
	ceiling_nodes: Vec<FloorNode>,
	/// Connectable openings that participated in wall mapping.
	openings: Openings,
	/// Contact geometry for mapped openings.
	mapped: MappedOpenings,
}

impl ArcFloor {
	pub fn builder(center_xz: Vec3, radius: f32, storey_height: f32) -> ArcFloorBuilder {
		ArcFloorBuilder::new(center_xz, radius, storey_height)
	}

	pub fn new(params: ArcFloorParams) -> Self {
		Self::from_params(params)
	}

	fn from_params(params: ArcFloorParams) -> Self {
		let radius = params.radius.max(1e-4);
		let storey_height = params.storey_height.max(1e-4);
		let center_xz = Vec3::new(params.center_xz.x, params.center_xz.y, params.center_xz.z);
		let params = ArcFloorParams {
			center_xz,
			radius,
			storey_height,
			..params
		};

		let (sectors, wall_partitions) = resolve_wall_sweeps(&params);
		let (openings, mapped) = map_connectable_openings(&params, &sectors);

		let floor_nodes = resolve_slab(
			params.floor,
			center_xz,
			radius,
			center_xz.y,
			&params.openings,
		);
		let ceiling_nodes = resolve_slab(
			params.ceiling,
			center_xz + Vec3::Y * storey_height,
			radius,
			center_xz.y + storey_height,
			&params.openings,
		);

		Self {
			params,
			wall_partitions,
			floor_nodes,
			ceiling_nodes,
			openings,
			mapped,
		}
	}

	/// Authoring helper: thin passage/aperture AABB on the ring at normalized \(t\).
	pub fn plan_opening_at_t(
		id: impl Into<OpeningId>,
		label: OpeningLabel,
		center_xz: Vec3,
		radius: f32,
		storey_height: f32,
		t: f32,
	) -> (OpeningId, Opening) {
		let id = id.into();
		let dir = ring_dir_at(t);
		let radius = radius.max(1e-4);
		let storey_height = storey_height.max(1e-4);
		let center = Vec3::new(center_xz.x, center_xz.y, center_xz.z);
		let on_ring = Vec3::new(
			center.x + dir.x * radius,
			center.y,
			center.z + dir.y * radius,
		);
		let right = Vec3::new(-dir.y, 0.0, dir.x);
		let half_w = radius * (SEG_DEG.to_radians() * 0.5).sin().max(0.15);
		let half_d = 0.35;
		let h = SLICE_Y_FRAC * storey_height;
		let min = on_ring - right * half_w - Vec3::new(dir.x, 0.0, dir.y) * half_d;
		let max = on_ring + right * half_w + Vec3::new(dir.x, 0.0, dir.y) * half_d + Vec3::Y * h;
		(
			id,
			Opening::new(Aabb3d::from_min_max(min.min(max), min.max(max)), label),
		)
	}

	pub fn params(&self) -> &ArcFloorParams {
		&self.params
	}

	pub fn wall_partitions(&self) -> &[PartitionNode] {
		&self.wall_partitions
	}

	pub fn floor_nodes(&self) -> &[FloorNode] {
		&self.floor_nodes
	}

	pub fn ceiling_nodes(&self) -> &[FloorNode] {
		&self.ceiling_nodes
	}

	/// One kit segment in unit \(t\) (15° / 360°).
	pub fn segment_t(&self) -> f32 {
		SEG_DEG / 360.0
	}

	/// Outward unit direction in XZ at normalized sweep parameter \(t\).
	pub fn ring_dir_at(&self, t: f32) -> Vec2 {
		ring_dir_at(t)
	}

	/// World point on the ring exterior at \(t\) (floor elevation).
	pub fn ring_point_at(&self, t: f32) -> Vec3 {
		let dir = self.ring_dir_at(t);
		let c = self.params.center_xz;
		let r = self.params.radius;
		Vec3::new(c.x + dir.x * r, c.y, c.z + dir.y * r)
	}
}

impl MapsOpenings for ArcFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening> {
		self.mapped.get(id)
	}
}

impl BuildingComponents for ArcFloor {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		if matches!(level, LodSceneLevel::High | LodSceneLevel::Medium) {
			Layers::from_free(self.wall_partitions.clone())
		} else {
			Layers::new()
		}
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		if !matches!(level, LodSceneLevel::High) {
			return Layers::new();
		}
		let mut nodes = self.floor_nodes.clone();
		nodes.extend(self.ceiling_nodes.iter().cloned());
		Layers::from_free(nodes)
	}
}

// ─── Layer 1: wall sweeps ───────────────────────────────────────────────────

fn resolve_wall_sweeps(params: &ArcFloorParams) -> ([SectorWall; SECTORS as usize], Vec<PartitionNode>) {
	let mut sectors = [SectorWall::Solid; SECTORS as usize];
	let band_y0 = params.center_xz.y;
	let lintel_y = band_y0 + SLICE_Y_FRAC * params.storey_height;
	let band_y1 = band_y0 + params.storey_height;

	for (_id, opening) in params.openings.iter() {
		for i in 0..SECTORS {
			let sector = sector_aabb(params, i);
			if !aabb3d_intersects(&opening.bounds, &sector) {
				continue;
			}
			let lower = band_aabb(&sector, band_y0, lintel_y);
			let upper = band_aabb(&sector, lintel_y, band_y1);
			let hit_lower = aabb3d_intersects(&opening.bounds, &lower);
			let hit_upper = aabb3d_intersects(&opening.bounds, &upper);
			if !hit_lower && !hit_upper {
				// Sector AABB was a false positive — keep solid.
				continue;
			}
			let idx = i as usize;
			sectors[idx] = match (hit_lower, hit_upper) {
				(true, true) => SectorWall::Empty,
				(true, false) => match sectors[idx] {
					SectorWall::Empty => SectorWall::Empty,
					_ => SectorWall::LintelOnly,
				},
				(false, true) => match sectors[idx] {
					SectorWall::LintelOnly | SectorWall::Empty => SectorWall::Empty,
					SectorWall::Solid => SectorWall::Solid, // upper-only: keep full solid strip
				},
				(false, false) => sectors[idx],
			};
		}
	}

	let partitions = emit_wall_partitions(params, &sectors);
	(sectors, partitions)
}

fn emit_wall_partitions(params: &ArcFloorParams, sectors: &[SectorWall; SECTORS as usize]) -> Vec<PartitionNode> {
	let ring_scale = Vec3::new(params.radius, params.storey_height, params.radius);
	let lintel = params.center_xz + Vec3::Y * (SLICE_Y_FRAC * params.storey_height);
	let mut partitions = Vec::new();

	// Emit lintel-only and empty as 15° pieces; merge solid runs into 180/90/15.
	let mut i = 0u32;
	while i < SECTORS {
		match sectors[i as usize] {
			SectorWall::Empty => {
				i += 1;
			}
			SectorWall::LintelOnly => {
				push_slice(
					&mut partitions,
					lintel,
					ring_scale,
					i as f32 * SEG_DEG,
					SEG_DEG,
					params.style,
				);
				i += 1;
			}
			SectorWall::Solid => {
				let mut run = 1u32;
				while i + run < SECTORS && sectors[(i + run) as usize] == SectorWall::Solid {
					run += 1;
				}
				emit_solid_run(
					&mut partitions,
					params.center_xz,
					ring_scale,
					i,
					run,
					params.style,
				);
				i += run;
			}
		}
	}
	partitions
}

fn emit_solid_run(
	partitions: &mut Vec<PartitionNode>,
	center_xz: Vec3,
	ring_scale: Vec3,
	start_sector: u32,
	run: u32,
	style: PartitionStyle,
) {
	let mut remaining = run;
	let mut at = start_sector;
	// Prefer 180°, then 90°, then 15° leftovers.
	// Kits sweep CW from their placement yaw, so a run of sectors
	// `at .. at+chunk` is covered by a kit whose yaw is the *last* sector's yaw.
	while remaining > 0 {
		let chunk = if remaining >= 12 {
			12 // 180°
		} else if remaining >= 6 {
			6 // 90°
		} else {
			1 // 15°
		};
		let yaw_sector = at + chunk - 1;
		push_solid(
			partitions,
			center_xz,
			ring_scale,
			yaw_sector as f32 * SEG_DEG,
			chunk as f32 * SEG_DEG,
			style,
		);
		at += chunk;
		remaining -= chunk;
	}
}

fn push_solid(
	partitions: &mut Vec<PartitionNode>,
	center_xz: Vec3,
	ring_scale: Vec3,
	start_deg: f32,
	sweep_deg: f32,
	style: PartitionStyle,
) {
	if sweep_deg > 1e-2 {
		partitions.push(PartitionNode::new(
			style,
			Partition::arc(sweep_deg),
			Placement::new(center_xz, start_deg.to_radians()).with_scale(ring_scale),
		));
	}
}

fn push_slice(
	partitions: &mut Vec<PartitionNode>,
	lintel: Vec3,
	ring_scale: Vec3,
	start_deg: f32,
	sweep_deg: f32,
	style: PartitionStyle,
) {
	if sweep_deg > 1e-2 {
		partitions.push(PartitionNode::new(
			style,
			Partition::slice_arc(sweep_deg),
			Placement::new(lintel, start_deg.to_radians()).with_scale(ring_scale),
		));
	}
}

/// AABB approximating the kit sector: yaw `sector·15°`, sweep CW through −Z.
fn sector_aabb(params: &ArcFloorParams, sector: u32) -> Aabb3d {
	let start = sector as f32 * SEG_DEG;
	// CW end is start − 15° (kit convention).
	let r_in = params.radius * 0.85;
	let r_out = params.radius * 1.05;
	let y0 = params.center_xz.y;
	let y1 = y0 + params.storey_height;
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	// Sample the CW wedge (including mid) so the AABB tracks the real arc.
	for step in 0..=2 {
		let deg = start - SEG_DEG * (step as f32 / 2.0);
		let d = ring_dir_at_deg(deg);
		for r in [r_in, r_out] {
			for y in [y0, y1] {
				let p = Vec3::new(
					params.center_xz.x + d.x * r,
					y,
					params.center_xz.z + d.y * r,
				);
				min = min.min(p);
				max = max.max(p);
			}
		}
	}
	Aabb3d::from_min_max(min, max)
}

fn band_aabb(sector: &Aabb3d, y0: f32, y1: f32) -> Aabb3d {
	let mut min = Vec3::from(sector.min);
	let mut max = Vec3::from(sector.max);
	min.y = y0.min(y1);
	max.y = y0.max(y1);
	Aabb3d::from_min_max(min, max)
}

fn map_connectable_openings(
	params: &ArcFloorParams,
	sectors: &[SectorWall; SECTORS as usize],
) -> (Openings, MappedOpenings) {
	let mut openings = Openings::new();
	let mut mapped = MappedOpenings::new();
	for (id, opening) in params.openings.iter() {
		if !opening.label.is_connectable() {
			continue;
		}
		let mut hit_sectors = Vec::new();
		for i in 0..SECTORS {
			if matches!(sectors[i as usize], SectorWall::Solid) {
				continue;
			}
			let sector = sector_aabb(params, i);
			if aabb3d_intersects(&opening.bounds, &sector) {
				hit_sectors.push(i);
			}
		}
		if hit_sectors.is_empty() {
			continue;
		}
		openings.insert(id.clone(), opening.clone());
		mapped.insert(id.clone(), mapped_from_sectors(params, &hit_sectors));
	}
	(openings, mapped)
}

fn mapped_from_sectors(params: &ArcFloorParams, hit: &[u32]) -> MappedOpening {
	let lo = *hit.iter().min().unwrap_or(&0);
	let hi = *hit.iter().max().unwrap_or(&0);
	// Sector i covers CW from i·15° to i·15°−15°. Contiguous hits: hi·15° CW to lo·15°−15°.
	let deg_start = hi as f32 * SEG_DEG;
	let deg_end = lo as f32 * SEG_DEG - SEG_DEG;
	let deg_mid = 0.5 * (deg_start + deg_end);
	let bl = ring_point_deg(params, deg_end);
	let br = ring_point_deg(params, deg_start);
	let h = SLICE_Y_FRAC * params.storey_height;
	let tl = bl + Vec3::Y * h;
	let tr = br + Vec3::Y * h;
	let orientation = ring_dir_at_deg(deg_mid);
	let right = Vec3::new(-orientation.y, 0.0, orientation.x);
	let (bl, br, tl, tr) = if (br - bl).dot(right) < 0.0 {
		(br, bl, tr, tl)
	} else {
		(bl, br, tl, tr)
	};
	MappedOpening::new(MappedOpeningQuad::new(bl, br, tl, tr), orientation)
}

// ─── Layer 2: floor / ceiling ───────────────────────────────────────────────

fn resolve_slab(
	base: ArcFloorSlab,
	center: Vec3,
	radius: f32,
	slab_y: f32,
	openings: &Openings,
) -> Vec<FloorNode> {
	match base {
		ArcFloorSlab::None => Vec::new(),
		ArcFloorSlab::Solid => {
			let max_side = 2.0 * INSCRIBED_HALF_FRAC * radius; // 1.4·R
			let slab_aabb = slab_volume_aabb(center, radius, slab_y);
			let mut hole_side: Option<f32> = None;
			let mut remove_all = false;
			for (_id, opening) in openings.iter() {
				if !opening.label.cuts_slab() {
					continue;
				}
				if !aabb3d_intersects(&opening.bounds, &slab_aabb) {
					continue;
				}
				let Some(inter) = aabb_intersection(&opening.bounds, &slab_aabb) else {
					continue;
				};
				let extent = Vec3::from(inter.max - inter.min);
				// Characteristic horizontal scale of the intersection.
				let scale = extent.x.max(extent.z);
				if scale + EPS >= max_side {
					remove_all = true;
					break;
				}
				hole_side = Some(hole_side.map_or(scale, |s| s.max(scale)));
			}
			if remove_all {
				Vec::new()
			} else if let Some(side) = hole_side {
				let inscribed_half = INSCRIBED_HALF_FRAC * radius;
				let hole_half = (side * 0.5).clamp(1e-4, inscribed_half * 0.95);
				let (caps, rects) = squared_floor_with_hole(center, radius, hole_half);
				let mut nodes = caps.to_vec();
				nodes.extend(rects);
				nodes
			} else {
				let mut nodes = inscribed_caps(center, radius);
				let side = max_side;
				nodes.push(rect_slab(center, side, side));
				nodes
			}
		}
	}
}

fn slab_volume_aabb(center: Vec3, radius: f32, slab_y: f32) -> Aabb3d {
	let half = radius.max(1e-4);
	let y_half = FLOOR_SLAB_Y_SCALE;
	Aabb3d::from_min_max(
		Vec3::new(center.x - half, slab_y - y_half, center.z - half),
		Vec3::new(center.x + half, slab_y + y_half, center.z + half),
	)
}

fn aabb_intersection(a: &Aabb3d, b: &Aabb3d) -> Option<Aabb3d> {
	if !aabb3d_intersects(a, b) {
		return None;
	}
	let min = Vec3::from(a.min).max(Vec3::from(b.min));
	let max = Vec3::from(a.max).min(Vec3::from(b.max));
	Some(Aabb3d::from_min_max(min, max))
}

fn aabb3d_intersects(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min.x < b.max.x - EPS
		&& a.max.x > b.min.x + EPS
		&& a.min.y < b.max.y - EPS
		&& a.max.y > b.min.y + EPS
		&& a.min.z < b.max.z - EPS
		&& a.max.z > b.min.z + EPS
}

fn inscribed_caps(center_xz: Vec3, radius: f32) -> Vec<FloorNode> {
	let radius = radius.max(1e-4);
	let ring_scale = Vec3::new(radius, FLOOR_SLAB_Y_SCALE, radius);
	vec![
		FloorNode::rough_stone(
			Floor::circle_inscribed_square(),
			Placement::new(center_xz, 0.0).with_scale(ring_scale),
		),
		FloorNode::rough_stone(
			Floor::circle_inscribed_square(),
			Placement::new(center_xz, std::f32::consts::FRAC_PI_2).with_scale(ring_scale),
		),
		FloorNode::rough_stone(
			Floor::circle_inscribed_square(),
			Placement::new(center_xz, std::f32::consts::PI).with_scale(ring_scale),
		),
		FloorNode::rough_stone(
			Floor::circle_inscribed_square(),
			Placement::new(center_xz, std::f32::consts::PI + std::f32::consts::FRAC_PI_2)
				.with_scale(ring_scale),
		),
	]
}

fn squared_floor_with_hole(
	center_xz: Vec3,
	radius: f32,
	hole_half: f32,
) -> ([FloorNode; 4], [FloorNode; 4]) {
	let radius = radius.max(1e-4);
	let inscribed_half = INSCRIBED_HALF_FRAC * radius;
	let hole_half = hole_half.clamp(1e-4, inscribed_half * 0.95);
	let caps = inscribed_caps(center_xz, radius);
	let caps = [caps[0].clone(), caps[1].clone(), caps[2].clone(), caps[3].clone()];
	let inscribed_side = 2.0 * inscribed_half;

	let cx = center_xz.x;
	let cz = center_xz.z;
	let y = center_xz.y;
	let inscribed_min_z = cz - inscribed_half;
	let inscribed_max_z = cz + inscribed_half;
	let inscribed_min_x = cx - inscribed_half;
	let inscribed_max_x = cx + inscribed_half;
	let hole_min_z = cz - hole_half;
	let hole_max_z = cz + hole_half;
	let hole_min_x = cx - hole_half;
	let hole_max_x = cx + hole_half;

	let gap_s = (hole_min_z - inscribed_min_z).max(0.0);
	let gap_n = (inscribed_max_z - hole_max_z).max(0.0);
	let gap_w = (hole_min_x - inscribed_min_x).max(0.0);
	let gap_e = (inscribed_max_x - hole_max_x).max(0.0);

	let south =
		rect_slab(Vec3::new(cx, y, 0.5 * (inscribed_min_z + hole_min_z)), inscribed_side, gap_s);
	let north =
		rect_slab(Vec3::new(cx, y, 0.5 * (hole_max_z + inscribed_max_z)), inscribed_side, gap_n);
	let west =
		rect_slab(Vec3::new(0.5 * (inscribed_min_x + hole_min_x), y, cz), gap_w, inscribed_side);
	let east =
		rect_slab(Vec3::new(0.5 * (hole_max_x + inscribed_max_x), y, cz), gap_e, inscribed_side);

	(caps, [south, north, west, east])
}

fn rect_slab(center: Vec3, width_x: f32, depth_z: f32) -> FloorNode {
	let width_x = width_x.max(1e-4);
	let depth_z = depth_z.max(1e-4);
	let origin = Vec3::new(center.x - 0.5 * width_x, center.y, center.z - 0.5 * depth_z);
	FloorNode::rough_stone(
		Floor::rectangle(),
		Placement::new(origin, 0.0).with_scale(Vec3::new(width_x, FLOOR_SLAB_Y_SCALE, depth_z)),
	)
}

fn ring_dir_at(t: f32) -> Vec2 {
	ring_dir_at_deg(norm_t(t) * 360.0)
}

fn ring_dir_at_deg(deg: f32) -> Vec2 {
	let phi = deg.to_radians();
	let (s, c) = phi.sin_cos();
	// Kit local −X after Bevy YXZ yaw: (−cos φ, sin φ).
	Vec2::new(-c, s)
}

fn ring_point_deg(params: &ArcFloorParams, deg: f32) -> Vec3 {
	let dir = ring_dir_at_deg(deg);
	Vec3::new(
		params.center_xz.x + dir.x * params.radius,
		params.center_xz.y,
		params.center_xz.z + dir.y * params.radius,
	)
}

fn norm_t(t: f32) -> f32 {
	let mut t = t % 1.0;
	if t < 0.0 {
		t += 1.0;
	}
	t
}

#[cfg(test)]
mod tests {
	use super::*;

	fn openings_at(ts_labels: &[(&str, f32, OpeningLabel)]) -> Openings {
		let mut openings = Openings::new();
		for (id, t, label) in ts_labels {
			let (id, opening) =
				ArcFloor::plan_opening_at_t(*id, label.clone(), Vec3::ZERO, 4.0, 3.0, *t);
			openings.insert(id, opening);
		}
		openings
	}

	#[test]
	fn openings_cut_wall_partitions() -> anyhow::Result<()> {
		let floor = ArcFloor::builder(Vec3::ZERO, 4.0, 3.0)
			.openings(openings_at(&[
				("door", 0.0, OpeningLabel::Passage),
				("window", 0.5, OpeningLabel::Aperture),
			]))
			.build();
		assert!(!floor.wall_partitions().is_empty());
		assert!(floor.openings().len() >= 1);
		Ok(())
	}

	#[test]
	fn slab_none_omits_nodes() -> anyhow::Result<()> {
		let floor = ArcFloor::builder(Vec3::ZERO, 4.0, 3.0)
			.floor(ArcFloorSlab::None)
			.ceiling(ArcFloorSlab::None)
			.build();
		assert!(floor.floor_nodes().is_empty());
		assert!(floor.ceiling_nodes().is_empty());
		Ok(())
	}

	#[test]
	fn solid_slab_without_openings() -> anyhow::Result<()> {
		let solid = ArcFloor::builder(Vec3::ZERO, 4.0, 3.0)
			.floor(ArcFloorSlab::Solid)
			.build();
		// 4 caps + 1 inscribed fill
		assert_eq!(solid.floor_nodes().len(), 5);
		Ok(())
	}

	#[test]
	fn large_floor_opening_removes_slab() -> anyhow::Result<()> {
		let r = 4.0;
		// Square AABB with half-length ≈ radius → removes entire Solid floor.
		let mut openings = Openings::new();
		openings.insert(
			"clear",
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(-r, -0.5, -r), Vec3::new(r, 0.5, r)),
				OpeningLabel::Shaft,
			),
		);
		let floor = ArcFloor::builder(Vec3::ZERO, r, 3.0)
			.floor(ArcFloorSlab::Solid)
			.openings(openings)
			.build();
		assert!(floor.floor_nodes().is_empty());
		Ok(())
	}

	#[test]
	fn mapped_opening_from_wall_hit() -> anyhow::Result<()> {
		let connect = OpeningId::new("connect");
		let floor = ArcFloor::builder(Vec3::ZERO, 4.0, 3.0)
			.openings(openings_at(&[("connect", 0.5, OpeningLabel::Passage)]))
			.build();
		let east = floor
			.mapped_opening(&connect)
			.ok_or_else(|| anyhow::anyhow!("missing mapped opening {connect:?}"))?;
		let orient = east.orientation.normalize();
		assert!(orient.x > 0.7, "east door should face +X, orient={orient:?}");
		let (bl, br, ..) = east.endpoint_corners();
		let mid = (bl + br) * 0.5;
		assert!(mid.x > 3.0, "mapped mid should sit on +X ring, mid={mid:?}");
		assert!(bl.distance(br) > 0.1);
		Ok(())
	}

	#[test]
	fn passage_does_not_cut_floor_slab() -> anyhow::Result<()> {
		let floor = ArcFloor::builder(Vec3::ZERO, 4.0, 3.0)
			.floor(ArcFloorSlab::Solid)
			.openings(openings_at(&[("door", 0.5, OpeningLabel::Passage)]))
			.build();
		// Solid fill with no slab-cutting openings: 4 caps + 1 inscribed rect.
		assert_eq!(floor.floor_nodes().len(), 5);
		Ok(())
	}

	#[test]
	fn east_door_does_not_drop_quarter_ring() -> anyhow::Result<()> {
		let floor = ArcFloor::builder(Vec3::ZERO, 4.0, 3.0)
			.openings(openings_at(&[("door", 0.5, OpeningLabel::Passage)]))
			.build();
		let solid_deg: f32 = floor
			.wall_partitions()
			.iter()
			.filter_map(|p| match &p.geometry {
				Partition::Arc(a) => Some(a.sweep_degrees),
				Partition::SliceArc(a) => Some(a.sweep_degrees),
				_ => None,
			})
			.sum();
		// Full ring is 360°; one door should remove well under a quarter of solid.
		assert!(
			solid_deg > 300.0,
			"unexpected missing wall mass: solid_deg={solid_deg}"
		);
		Ok(())
	}

	#[test]
	fn solid_runs_prefer_large_sweeps() -> anyhow::Result<()> {
		// No openings → one 180 + leftover merge into large arcs (12+12 sectors).
		let floor = ArcFloor::builder(Vec3::ZERO, 4.0, 3.0).build();
		let arcs = floor
			.wall_partitions()
			.iter()
			.filter(|p| matches!(p.geometry, Partition::Arc(_)))
			.count();
		assert!(arcs <= 4, "expected few merged solids, got {arcs}");
		assert!(!floor.wall_partitions().is_empty());
		Ok(())
	}
}
