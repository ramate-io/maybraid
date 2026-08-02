//! One circular storey shell: portal ring wall plus optional floor / ceiling slabs.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::floors::{Floor, FloorNode};
use richmond_building_components::partitions::{PartitionNode, PartitionStyle};
use richmond_building_components::{BuildingComponents, Layers, Placement};

use crate::arcs::{portal_ring_wall, PortalRingParams, PortalRingWall};
use crate::openings::{
	MappedOpening, MappedOpeningQuad, MappedOpenings, MapsOpenings, Opening, OpeningId,
	OpeningLabel, Openings,
};
use crate::portals::{MustAssignPortal, Portal, SLICE_Y_FRAC};

/// Kit segment size (degrees). Portal clips span two segments (30°); the visible
/// omitted opening is **one** segment (15°).
const SEG_DEG: f32 = 15.0;
const PORTAL_SEGS: u32 = 2;
const OPEN_HALF_DEG: f32 = SEG_DEG * (PORTAL_SEGS as f32) * 0.5;
/// Ring samples are 7.5° clockwise of raw `start_yaw + t·2π` so \(t\) hits a
/// segment edge (jamb), not the opening center.
const RING_CLOCKWISE_OFFSET_DEG: f32 = SEG_DEG * 0.5;

/// Inscribed-square half-extent as a fraction of outer radius.
const INSCRIBED_HALF_FRAC: f32 = 0.7;
/// Floor / ceiling slab Y scale.
const FLOOR_SLAB_Y_SCALE: f32 = 0.2;

/// Horizontal storey slab presentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArcFloorSlab {
	/// Omit the slab entirely.
	None,
	/// Squared floor fill (inscribed-square caps + solid inscribed square).
	Solid,
	/// Caps plus rectangular frame around a centered square hole; `size` is full side length.
	SquareHole { size: f32 },
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
	/// World yaw (radians) of the ring sweep start (\(t = 0\)).
	pub start_yaw: f32,
	/// Opening plan (Passage / Aperture projected onto the ring; no optional / noise portals).
	pub openings: Openings,
	pub floor: ArcFloorSlab,
	pub ceiling: ArcFloorSlab,
	pub style: PartitionStyle,
}

impl Default for ArcFloorParams {
	fn default() -> Self {
		Self {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			start_yaw: 0.0,
			openings: Openings::new(),
			floor: ArcFloorSlab::None,
			ceiling: ArcFloorSlab::None,
			style: PartitionStyle::RoughStonework,
		}
	}
}

/// One circular storey: clipped ring wall + optional floor / ceiling.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcFloor {
	params: ArcFloorParams,
	ring_wall: PortalRingWall,
	floor_nodes: Vec<FloorNode>,
	ceiling_nodes: Vec<FloorNode>,
	/// Portal half-width in unit \(t\) for this ring.
	half_t: f32,
	/// Connectable openings honored by this storey.
	openings: Openings,
	/// Contact geometry for honored openings.
	mapped: MappedOpenings,
}

impl ArcFloor {
	pub fn new(params: ArcFloorParams) -> Self {
		let radius = params.radius.max(1e-4);
		let storey_height = params.storey_height.max(1e-4);
		let center_xz = Vec3::new(params.center_xz.x, params.center_xz.y, params.center_xz.z);
		let arc_degrees = 360.0;
		let half_t = OPEN_HALF_DEG / arc_degrees;

		let mut must_assign = Vec::new();
		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		let mut assigned_ts: Vec<(OpeningId, f32, Opening)> = Vec::new();

		for (id, opening) in params.openings.iter() {
			let portal = match opening.label {
				OpeningLabel::Passage => Portal::Door,
				OpeningLabel::Aperture => Portal::Window,
				_ => continue,
			};
			let t = project_opening_t(center_xz, params.start_yaw, &opening.bounds);
			must_assign.push(MustAssignPortal::at(t, portal));
			assigned_ts.push((id.clone(), t, opening.clone()));
		}

		let ring_wall = portal_ring_wall(PortalRingParams {
			center_xz,
			radius,
			storey_height,
			arc_degrees,
			start_yaw: params.start_yaw,
			must_assign,
			must_not_assign: vec![],
			portal_noise: NoiseParams::default(),
			optional_portals: (0, 0),
			style: params.style,
		});

		let floor_nodes = build_slab(center_xz, radius, params.floor);
		let ceiling_center = center_xz + Vec3::Y * storey_height;
		let ceiling_nodes = build_slab(ceiling_center, radius, params.ceiling);

		let mut floor = Self {
			params: ArcFloorParams {
				center_xz,
				radius,
				storey_height,
				..params
			},
			ring_wall,
			floor_nodes,
			ceiling_nodes,
			half_t,
			openings: Openings::new(),
			mapped: MappedOpenings::new(),
		};

		for (id, t, opening) in assigned_ts {
			let assigned = floor
				.ring_wall
				.portals
				.iter()
				.any(|p| circular_dist(p.t, t) < 1e-3);
			if !assigned {
				continue;
			}
			openings.insert(id.clone(), opening);
			mapped.insert(id, floor.mapped_at(t));
		}
		floor.openings = openings;
		floor.mapped = mapped;
		floor
	}

	/// Authoring helper: thin passage/aperture AABB on the ring at normalized \(t\).
	pub fn plan_opening_at_t(
		id: impl Into<OpeningId>,
		label: OpeningLabel,
		center_xz: Vec3,
		radius: f32,
		storey_height: f32,
		start_yaw: f32,
		t: f32,
	) -> (OpeningId, Opening) {
		let id = id.into();
		let dir = ring_dir_at_yaw(start_yaw, t);
		let radius = radius.max(1e-4);
		let storey_height = storey_height.max(1e-4);
		let center = Vec3::new(center_xz.x, center_xz.y, center_xz.z);
		let on_ring = Vec3::new(
			center.x + dir.x * radius,
			center.y,
			center.z + dir.y * radius,
		);
		let right = Vec3::new(-dir.y, 0.0, dir.x);
		let half_w = radius * (OPEN_HALF_DEG.to_radians()).sin().max(0.15);
		let half_d = 0.35;
		let h = SLICE_Y_FRAC * storey_height;
		let min = on_ring - right * half_w - Vec3::new(dir.x, 0.0, dir.y) * half_d;
		let max = on_ring + right * half_w + Vec3::new(dir.x, 0.0, dir.y) * half_d + Vec3::Y * h;
		(
			id,
			Opening::new(
				Aabb3d::from_min_max(min.min(max), min.max(max)),
				label,
			),
		)
	}

	pub fn params(&self) -> &ArcFloorParams {
		&self.params
	}

	pub fn ring_wall(&self) -> &PortalRingWall {
		&self.ring_wall
	}

	pub fn floor_nodes(&self) -> &[FloorNode] {
		&self.floor_nodes
	}

	pub fn ceiling_nodes(&self) -> &[FloorNode] {
		&self.ceiling_nodes
	}

	/// Portal clip half-width in unit \(t\) (two segments → 15° each side of center).
	pub fn half_t(&self) -> f32 {
		self.half_t
	}

	/// One kit segment in unit \(t\) (15° / 360°).
	pub fn segment_t(&self) -> f32 {
		SEG_DEG / 360.0
	}

	/// Outward unit direction in XZ at normalized sweep parameter \(t\).
	///
	/// `start_yaw + t·2π` plus a **+7.5° clockwise** bias so grid samples sit on
	/// segment edges (jambs), not opening centers. Dir is kit local \(−X\) after
	/// Bevy `YXZ` yaw: `(-cos φ, sin φ)`.
	pub fn ring_dir_at(&self, t: f32) -> Vec2 {
		ring_dir_at_yaw(self.params.start_yaw, t)
	}

	/// World point on the ring exterior at \(t\) (floor elevation).
	///
	/// Exact: \(\texttt{center} + R · \mathrm{dir}(t)\) — no chord/tangent estimate.
	pub fn ring_point_at(&self, t: f32) -> Vec3 {
		let dir = self.ring_dir_at(t);
		let c = self.params.center_xz;
		let r = self.params.radius;
		Vec3::new(c.x + dir.x * r, c.y, c.z + dir.y * r)
	}

	/// Opening map at normalized \(t\) (does not require an assigned portal).
	///
	/// Visible door = **one 15° segment**. Plan-view “clockwise” along the ring
	/// (toward \(+Z\) from \(+X\) with our yaw map) is **decreasing** \(t\), so the
	/// segment one node clockwise of portal \(t\) is `t−30°` → `t−15°`.
	/// Tops = bottoms + slice height.
	fn mapped_at(&self, t: f32) -> MappedOpening {
		let t = norm_t(t);
		let seg = self.segment_t();
		// One segment clockwise of `[t−15°, t]` → `[t−30°, t−15°]`.
		let t_lo = norm_t(t - 2.0 * seg);
		let t_hi = norm_t(t - seg);
		let t_mid = norm_t(t - 1.5 * seg);
		// Looking outward: left is further clockwise (lower t / toward +Z at +X).
		let bl = self.ring_point_at(t_lo);
		let br = self.ring_point_at(t_hi);
		let h = SLICE_Y_FRAC * self.params.storey_height;
		let tl = bl + Vec3::Y * h;
		let tr = br + Vec3::Y * h;
		let orientation = self.ring_dir_at(t_mid);
		MappedOpening::new(MappedOpeningQuad::new(bl, br, tl, tr), orientation)
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
			self.ring_wall.sweep.partition_nodes_for_level(level)
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

fn build_slab(center: Vec3, radius: f32, slab: ArcFloorSlab) -> Vec<FloorNode> {
	match slab {
		ArcFloorSlab::None => Vec::new(),
		ArcFloorSlab::Solid => {
			let mut nodes = inscribed_caps(center, radius);
			let inscribed_half = INSCRIBED_HALF_FRAC * radius;
			let side = 2.0 * inscribed_half;
			nodes.push(rect_slab(center, side, side));
			nodes
		}
		ArcFloorSlab::SquareHole { size } => {
			let inscribed_half = INSCRIBED_HALF_FRAC * radius;
			let hole_half = (size * 0.5).clamp(1e-4, inscribed_half * 0.95);
			let (caps, rects) = squared_floor_with_hole(center, radius, hole_half);
			let mut nodes = caps.to_vec();
			nodes.extend(rects);
			nodes
		}
	}
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

fn norm_t(t: f32) -> f32 {
	let mut t = t % 1.0;
	if t < 0.0 {
		t += 1.0;
	}
	t
}

fn circular_dist(a: f32, b: f32) -> f32 {
	let d = (norm_t(a) - norm_t(b)).abs();
	d.min(1.0 - d)
}

fn ring_dir_at_yaw(start_yaw: f32, t: f32) -> Vec2 {
	let phi = start_yaw + norm_t(t) * std::f32::consts::TAU + RING_CLOCKWISE_OFFSET_DEG.to_radians();
	let (s, c) = phi.sin_cos();
	Vec2::new(-c, s)
}

/// Invert [`ring_dir_at_yaw`]: map a world XZ point to nearest unit \(t\) on the ring.
fn project_opening_t(center_xz: Vec3, start_yaw: f32, bounds: &Aabb3d) -> f32 {
	let mid = Vec3::from((bounds.min + bounds.max) * 0.5);
	let dx = mid.x - center_xz.x;
	let dz = mid.z - center_xz.z;
	if dx * dx + dz * dz < 1e-10 {
		return 0.0;
	}
	// dir = (-cos φ, sin φ) ⇒ φ = atan2(sin, cos) = atan2(dz, -dx)
	let phi = dz.atan2(-dx);
	norm_t((phi - start_yaw - RING_CLOCKWISE_OFFSET_DEG.to_radians()) / std::f32::consts::TAU)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn openings_at(ts_labels: &[(&str, f32, OpeningLabel)]) -> Openings {
		let mut openings = Openings::new();
		for (id, t, label) in ts_labels {
			let (id, opening) = ArcFloor::plan_opening_at_t(
				*id,
				label.clone(),
				Vec3::ZERO,
				4.0,
				3.0,
				0.0,
				*t,
			);
			openings.insert(id, opening);
		}
		openings
	}

	#[test]
	fn openings_assign_without_optional_noise() {
		let floor = ArcFloor::new(ArcFloorParams {
			openings: openings_at(&[
				("door", 0.0, OpeningLabel::Passage),
				("window", 0.5, OpeningLabel::Aperture),
			]),
			..ArcFloorParams::default()
		});
		assert_eq!(floor.ring_wall().portals.len(), 2);
		assert!(!floor.ring_wall().sweep.clip_intervals.is_empty());
		assert_eq!(floor.openings().len(), 2);
	}

	#[test]
	fn slab_none_omits_nodes() {
		let floor = ArcFloor::new(ArcFloorParams {
			floor: ArcFloorSlab::None,
			ceiling: ArcFloorSlab::None,
			..ArcFloorParams::default()
		});
		assert!(floor.floor_nodes().is_empty());
		assert!(floor.ceiling_nodes().is_empty());
	}

	#[test]
	fn solid_and_hole_slabs() {
		let solid = ArcFloor::new(ArcFloorParams {
			floor: ArcFloorSlab::Solid,
			..ArcFloorParams::default()
		});
		// 4 caps + 1 inscribed fill
		assert_eq!(solid.floor_nodes().len(), 5);

		let holed = ArcFloor::new(ArcFloorParams {
			floor: ArcFloorSlab::SquareHole { size: 2.0 },
			ceiling: ArcFloorSlab::Solid,
			storey_height: 3.0,
			..ArcFloorParams::default()
		});
		// 4 caps + 4 frame rects
		assert_eq!(holed.floor_nodes().len(), 8);
		assert_eq!(holed.ceiling_nodes().len(), 5);
		assert!((holed.ceiling_nodes()[0].placement.translation.y - 3.0).abs() < 1e-3);
	}

	#[test]
	fn mapped_opening_kit_angle_map() {
		let connect = OpeningId::new("connect");
		let floor = ArcFloor::new(ArcFloorParams {
			openings: openings_at(&[
				("north", 0.0, OpeningLabel::Passage),
				("window", 0.25, OpeningLabel::Aperture),
				("connect", 0.5, OpeningLabel::Passage),
			]),
			radius: 4.0,
			start_yaw: 0.0,
			center_xz: Vec3::ZERO,
			..ArcFloorParams::default()
		});
		// +7.5° clockwise bias: t=0.5 near +X/−Z (a jamb node, not opening mid).
		let at_door = floor.ring_dir_at(0.5);
		assert!(at_door.x > 0.9, "t=0.5 ~+X, got {at_door:?}");
		assert!(at_door.y < -0.05, "clockwise of +X → −Z, got {at_door:?}");

		// Door: one segment clockwise of t → `[t−30°, t−15°]` (decreasing t).
		let east = floor.mapped_opening(&connect).expect("connect");
		let (bl, br, tl, tr) = east.endpoint_corners();
		let seg = floor.segment_t();
		let expect_bl = floor.ring_point_at(0.5 - 2.0 * seg);
		let expect_br = floor.ring_point_at(0.5 - seg);
		assert!(bl.distance(expect_bl) < 1e-4, "bl={bl:?} expect={expect_bl:?}");
		assert!(br.distance(expect_br) < 1e-4, "br={br:?} expect={expect_br:?}");
		let r_bl = (bl.x * bl.x + bl.z * bl.z).sqrt();
		let r_br = (br.x * br.x + br.z * br.z).sqrt();
		assert!((r_bl - 4.0).abs() < 1e-3, "bl not on ring r={r_bl}");
		assert!((r_br - 4.0).abs() < 1e-3, "br not on ring r={r_br}");
		let h = crate::portals::SLICE_Y_FRAC * 3.0;
		assert!((tl.y - bl.y - h).abs() < 1e-3, "top is slice-height lift, dy={}", tl.y - bl.y);
		assert!((tr.y - br.y - h).abs() < 1e-3);
		let chord = bl.distance(br);
		let expect_chord = 2.0 * 4.0 * (7.5_f32).to_radians().sin();
		assert!(
			(chord - expect_chord).abs() < 1e-3,
			"chord={chord} expect 15° span {expect_chord}"
		);
		assert!(east.orientation.normalize().x > 0.7, "orient={:?}", east.orientation);
		// Mid toward +Z of +X (clockwise from +X in plan).
		let mid = (bl + br) * 0.5;
		assert!(mid.z > 0.5, "mid={mid:?}");
	}
}
