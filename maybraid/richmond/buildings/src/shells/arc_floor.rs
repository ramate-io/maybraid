//! One circular storey shell: portal ring wall plus optional floor / ceiling slabs.

use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::floors::{Floor, FloorNode};
use richmond_building_components::partitions::{PartitionNode, PartitionStyle};
use richmond_building_components::{BuildingComponents, Layers, Placement};

use crate::arcs::{portal_ring_wall, PortalRingParams, PortalRingWall};
use crate::portals::{MustAssignPortal, SLICE_Y_FRAC};
use crate::shells::connecting_hall::ConnectingHallEndpoint;

/// Kit segment size (degrees); portal width is two segments → 30°.
const SEG_DEG: f32 = 15.0;
const PORTAL_SEGS: u32 = 2;
const OPEN_HALF_DEG: f32 = SEG_DEG * (PORTAL_SEGS as f32) * 0.5;

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
	/// Explicit openings (no optional / noise portals).
	pub openings: Vec<MustAssignPortal>,
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
			openings: Vec::new(),
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
}

impl ArcFloor {
	pub fn new(params: ArcFloorParams) -> Self {
		let radius = params.radius.max(1e-4);
		let storey_height = params.storey_height.max(1e-4);
		let center_xz = Vec3::new(params.center_xz.x, params.center_xz.y, params.center_xz.z);
		let arc_degrees = 360.0;
		let half_t = OPEN_HALF_DEG / arc_degrees;

		let ring_wall = portal_ring_wall(PortalRingParams {
			center_xz,
			radius,
			storey_height,
			arc_degrees,
			start_yaw: params.start_yaw,
			must_assign: params.openings.clone(),
			must_not_assign: vec![],
			portal_noise: NoiseParams::default(),
			optional_portals: (0, 0),
			style: params.style,
		});

		let floor_nodes = build_slab(center_xz, radius, params.floor);
		let ceiling_center = center_xz + Vec3::Y * storey_height;
		let ceiling_nodes = build_slab(ceiling_center, radius, params.ceiling);

		Self {
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
		}
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

	/// Opening quad for a portal centered at normalized \(t\), looking outward.
	///
	/// Height spans the clipped opening (ground to lintel baseline). Returns `None` when
	/// no assigned portal lies near `t`.
	pub fn portal_endpoint(&self, t: f32) -> Option<ConnectingHallEndpoint> {
		let t = norm_t(t);
		let assigned = self
			.ring_wall
			.portals
			.iter()
			.find(|p| circular_dist(p.t, t) < 1e-3)?;
		Some(self.endpoint_at(assigned.t))
	}

	/// Portal half-width in unit \(t\).
	pub fn half_t(&self) -> f32 {
		self.half_t
	}

	/// Outward unit direction in XZ at normalized sweep parameter \(t\).
	///
	/// Kit local \(−X\) after Bevy `YXZ` yaw `start_yaw + t·2π`:
	/// `(-cos φ, sin φ)` in XZ. With `start_yaw = 0`:
	/// \(t=0→−X\), \(t=0.25→+Z\), \(t=0.5→+X\).
	pub fn ring_dir_at(&self, t: f32) -> Vec2 {
		let phi = self.params.start_yaw + norm_t(t) * std::f32::consts::TAU;
		let (s, c) = phi.sin_cos();
		Vec2::new(-c, s)
	}

	/// World point on the ring exterior at \(t\) (floor elevation).
	pub fn ring_point_at(&self, t: f32) -> Vec3 {
		let dir = self.ring_dir_at(t);
		let c = self.params.center_xz;
		let r = self.params.radius;
		Vec3::new(c.x + dir.x * r, c.y, c.z + dir.y * r)
	}

	/// Opening quad at normalized \(t\) without requiring an assigned portal.
	///
	/// Built in the **tangent plane** at [`Self::ring_point_at`] so the flat hall
	/// opening is centered on the door (not a chord inset/skewed by the arc).
	pub fn endpoint_at(&self, t: f32) -> ConnectingHallEndpoint {
		let t = norm_t(t);
		let half_angle = self.half_t * std::f32::consts::TAU;
		let orientation = self.ring_dir_at(t);
		// Looking outward: same right as [`ConnectingHallEndpoint`] (`(−o_z, o_x)` in XZ).
		let right = Vec3::new(-orientation.y, 0.0, orientation.x);
		let c = self.params.center_xz;
		let y0 = c.y;
		let y1 = y0 + SLICE_Y_FRAC * self.params.storey_height;
		// Half-width of the tangent-plane door matching the angular clip rays.
		let half_w = self.params.radius * half_angle.tan();
		let mid0 = self.ring_point_at(t);
		let mid1 = Vec3::new(mid0.x, y1, mid0.z);
		let bl = mid0 - right * half_w;
		let br = mid0 + right * half_w;
		let tl = mid1 - right * half_w;
		let tr = mid1 + right * half_w;
		ConnectingHallEndpoint::new(bl, br, tl, tr, orientation)
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::portals::Portal;

	#[test]
	fn openings_assign_without_optional_noise() {
		let floor = ArcFloor::new(ArcFloorParams {
			openings: vec![
				MustAssignPortal::at(0.0, Portal::Door),
				MustAssignPortal::at(0.5, Portal::Window),
			],
			..ArcFloorParams::default()
		});
		assert_eq!(floor.ring_wall().portals.len(), 2);
		assert!(!floor.ring_wall().sweep.clip_intervals.is_empty());
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
	fn portal_endpoint_kit_angle_map() {
		let floor = ArcFloor::new(ArcFloorParams {
			openings: vec![
				MustAssignPortal::at(0.0, Portal::Door),
				MustAssignPortal::at(0.25, Portal::Window),
				MustAssignPortal::at(0.5, Portal::Door),
			],
			radius: 4.0,
			start_yaw: 0.0,
			center_xz: Vec3::ZERO,
			..ArcFloorParams::default()
		});
		// t=0 → −X (kit local −X at yaw 0).
		let west = floor.portal_endpoint(0.0).expect("t=0");
		assert!(west.orientation.normalize().x < -0.9, "t=0 → −X, got {:?}", west.orientation);
		let mid_w = (west.targets.0 + west.targets.1) * 0.5;
		assert!((mid_w.x + 4.0).abs() < 1e-3 && mid_w.z.abs() < 1e-3, "{mid_w:?}");

		// t=0.25 → +Z.
		let north = floor.portal_endpoint(0.25).expect("t=0.25");
		assert!(north.orientation.normalize().y > 0.9, "t=0.25 → +Z, got {:?}", north.orientation);

		// t=0.5 → +X.
		let east = floor.portal_endpoint(0.5).expect("t=0.5");
		assert!(east.orientation.normalize().x > 0.9, "t=0.5 → +X, got {:?}", east.orientation);
		let mid_e = (east.targets.0 + east.targets.1) * 0.5;
		assert!((mid_e.x - 4.0).abs() < 1e-3 && mid_e.z.abs() < 1e-3, "{mid_e:?}");
	}
}
