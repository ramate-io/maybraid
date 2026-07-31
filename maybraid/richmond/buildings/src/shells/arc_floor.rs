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

	/// Opening quad at normalized \(t\) without requiring an assigned portal.
	pub fn endpoint_at(&self, t: f32) -> ConnectingHallEndpoint {
		let t = norm_t(t);
		let sweep_rad = std::f32::consts::TAU;
		let half_angle = self.half_t * sweep_rad;
		let mid_yaw = self.params.start_yaw + t * sweep_rad;
		// Looking outward: left is +t (see spiral / ring yaw convention).
		let left_yaw = mid_yaw + half_angle;
		let right_yaw = mid_yaw - half_angle;
		let y0 = self.params.center_xz.y;
		let y1 = y0 + SLICE_Y_FRAC * self.params.storey_height;
		let r = self.params.radius;
		let c = self.params.center_xz;
		let bl = point_on_ring(c, r, left_yaw, y0);
		let br = point_on_ring(c, r, right_yaw, y0);
		let tl = point_on_ring(c, r, left_yaw, y1);
		let tr = point_on_ring(c, r, right_yaw, y1);
		let (s, cos) = mid_yaw.sin_cos();
		let orientation = Vec2::new(cos, -s);
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

/// Ring point using the same yaw convention as spiral stairs / arc kits:
/// \(θ = 0 → (+R, 0)\), increasing \(θ\) toward \(−Z\).
fn point_on_ring(center: Vec3, radius: f32, yaw: f32, y: f32) -> Vec3 {
	let (s, c) = yaw.sin_cos();
	Vec3::new(center.x + c * radius, y, center.z - s * radius)
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
	fn portal_endpoint_faces_plus_x_at_t0() {
		let floor = ArcFloor::new(ArcFloorParams {
			openings: vec![MustAssignPortal::at(0.0, Portal::Door)],
			radius: 4.0,
			start_yaw: 0.0,
			..ArcFloorParams::default()
		});
		let end = floor.portal_endpoint(0.0).expect("door");
		let o = end.orientation.normalize();
		assert!(o.x > 0.9, "orientation={o:?}");
		assert!(o.y.abs() < 0.1);
	}
}
