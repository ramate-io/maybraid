//! One-kink hall connecting two oriented openings via a [`Tube`].
//!
//! Each endpoint supplies an opening quad and an XZ facing direction (usually the
//! opening normal — not necessarily aimed at the other end). Rays along those
//! orientations meet in plan; the junction height and cross-section are
//! length-weighted lerps of the two ends.

use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::tube::{Tube, TubeCrossSectionNode, TubeFaces};

const EPS: f32 = 1e-5;

/// One end of a [`ConnectingHall`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectingHallEndpoint {
	/// `(bottom_left, bottom_right, top_left, top_right)` looking along [`Self::orientation`].
	pub targets: (Vec3, Vec3, Vec3, Vec3),
	/// Opening facing direction in XZ (need not point at the other end).
	pub orientation: Vec2,
}

impl ConnectingHallEndpoint {
	pub fn new(
		bottom_left: Vec3,
		bottom_right: Vec3,
		top_left: Vec3,
		top_right: Vec3,
		orientation: Vec2,
	) -> Self {
		Self {
			targets: (bottom_left, bottom_right, top_left, top_right),
			orientation,
		}
	}

	/// Expand the opening horizontally past the door jambs by `side_overrun` meters each side.
	///
	/// Overrunning reads better than stopping short or going too narrow when connecting
	/// shells backwards from openings.
	///
	/// Expansion is from the opening midline along ±[`Self::orientation`]'s right, so it
	/// stays centered even if the authored corners were left/right swapped (e.g. a
	/// face authored looking inward at the wall).
	pub fn widened(self, side_overrun: f32) -> Self {
		let overrun = side_overrun.max(0.0);
		let Some(orient) = normalize_xz(self.orientation) else {
			return self;
		};
		let right = Vec3::new(-orient.y, 0.0, orient.x);
		let (bl, br, tl, tr) = self.targets;
		let bottom_mid = (bl + br) * 0.5;
		let top_mid = (tl + tr) * 0.5;
		let half_b = 0.5 * bl.distance(br) + overrun;
		let half_t = 0.5 * tl.distance(tr) + overrun;
		Self {
			targets: (
				bottom_mid - right * half_b,
				bottom_mid + right * half_b,
				top_mid - right * half_t,
				top_mid + right * half_t,
			),
			orientation: self.orientation,
		}
	}
}

/// Small connector: two openings → one-kink plan path → [`Tube`].
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectingHall {
	style: PanelStyle,
	end_a: ConnectingHallEndpoint,
	end_b: ConnectingHallEndpoint,
	midpoint: Vec3,
	stations: [TubeCrossSectionNode; 3],
	tube: Tube,
}

impl ConnectingHall {
	pub fn new(
		style: PanelStyle,
		end_a: ConnectingHallEndpoint,
		end_b: ConnectingHallEndpoint,
	) -> Self {
		match build_stations(end_a, end_b) {
			Some((midpoint, stations)) => {
				let tube = Tube::from_nodes(style, stations);
				Self {
					style,
					end_a,
					end_b,
					midpoint,
					stations,
					tube,
				}
			}
			None => {
				debug_assert!(
					false,
					"ConnectingHall: orientation rays do not meet in plan"
				);
				Self {
					style,
					end_a,
					end_b,
					midpoint: Vec3::ZERO,
					stations: [TubeCrossSectionNode::new(Vec3::ZERO, 0.0, 0.0, 0.0, 0.0, 0.0); 3],
					tube: Tube::new(style),
				}
			}
		}
	}

	pub fn rough_stone(end_a: ConnectingHallEndpoint, end_b: ConnectingHallEndpoint) -> Self {
		Self::new(PanelStyle::RoughStonework, end_a, end_b)
	}

	pub fn with_faces(mut self, faces: TubeFaces) -> Self {
		self.tube = std::mem::replace(&mut self.tube, Tube::new(self.style)).with_faces(faces);
		self
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.tube =
			std::mem::replace(&mut self.tube, Tube::new(self.style)).with_joint_policy(joint_policy);
		self
	}

	pub fn tube(&self) -> &Tube {
		&self.tube
	}

	pub fn midpoint(&self) -> Vec3 {
		self.midpoint
	}

	pub fn endpoints(&self) -> (ConnectingHallEndpoint, ConnectingHallEndpoint) {
		(self.end_a, self.end_b)
	}

	pub fn stations(&self) -> &[TubeCrossSectionNode; 3] {
		&self.stations
	}
}

impl BuildingComponents for ConnectingHall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.tube.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.tube.joint_nodes_for_level(level)
	}
}

fn build_stations(
	end_a: ConnectingHallEndpoint,
	end_b: ConnectingHallEndpoint,
) -> Option<(Vec3, [TubeCrossSectionNode; 3])> {
	let node_a = endpoint_to_node(end_a)?;
	let node_b = endpoint_to_node(end_b)?;

	let p_a = Vec2::new(node_a.bottom_middle.x, node_a.bottom_middle.z);
	let p_b = Vec2::new(node_b.bottom_middle.x, node_b.bottom_middle.z);
	let d_a = normalize_xz(end_a.orientation)?;
	let d_b = normalize_xz(end_b.orientation)?;

	// Prefer the plan kink where the opening rays meet. When openings are skewed
	// (e.g. an arc-door chord offset from the facing axis) the forward rays may
	// miss — fall back to the midpoint between the openings.
	let m_xz = match ray_intersect_xz(p_a, d_a, p_b, d_b) {
		Some((t, s, m)) if t >= -EPS && s >= -EPS => m,
		_ => (p_a + p_b) * 0.5,
	};

	let l_a = (m_xz - p_a).length().max(EPS);
	let l_b = (m_xz - p_b).length().max(EPS);
	let inv = 1.0 / (l_a + l_b);
	let w_a = l_b * inv;
	let w_b = l_a * inv;

	let h_m = w_a * node_a.bottom_middle.y + w_b * node_b.bottom_middle.y;
	let mid = Vec3::new(m_xz.x, h_m, m_xz.y);
	let node_mid = lerp_nodes(node_a, node_b, w_a, w_b, mid);

	Some((mid, [node_a, node_mid, node_b]))
}

fn endpoint_to_node(end: ConnectingHallEndpoint) -> Option<TubeCrossSectionNode> {
	let (bl, br, tl, tr) = end.targets;
	let orient = normalize_xz(end.orientation)?;
	let right = Vec3::new(-orient.y, 0.0, orient.x);

	let bottom_middle = (bl + br) * 0.5;
	let top_middle = (tl + tr) * 0.5;
	// Vertical span for mid-station lerp; pitched offset is carried by `top_middle`.
	let height = (top_middle.y - bottom_middle.y).abs().max(EPS);

	let bottom_left_width = signed_width(bl, bottom_middle, right);
	let bottom_right_width = signed_width(br, bottom_middle, right);
	let top_left_width = signed_width(tl, top_middle, right);
	let top_right_width = signed_width(tr, top_middle, right);

	Some(
		TubeCrossSectionNode::new(
			bottom_middle,
			bottom_left_width,
			bottom_right_width,
			height,
			top_left_width,
			top_right_width,
		)
		.with_top_middle(top_middle),
	)
}

fn signed_width(corner: Vec3, middle: Vec3, right: Vec3) -> f32 {
	let d = corner - middle;
	let along = d.dot(right);
	// Widths are positive extents along ±right from middle.
	along.abs().max(0.0)
}

fn lerp_nodes(
	a: TubeCrossSectionNode,
	b: TubeCrossSectionNode,
	w_a: f32,
	w_b: f32,
	bottom_middle: Vec3,
) -> TubeCrossSectionNode {
	let mut mid = TubeCrossSectionNode::new(
		bottom_middle,
		w_a * a.bottom_left_width + w_b * b.bottom_left_width,
		w_a * a.bottom_right_width + w_b * b.bottom_right_width,
		w_a * a.height + w_b * b.height,
		w_a * a.top_left_width + w_b * b.top_left_width,
		w_a * a.top_right_width + w_b * b.top_right_width,
	);
	match (a.top_middle, b.top_middle) {
		(Some(ta), Some(tb)) => {
			mid = mid.with_top_middle(ta * w_a + tb * w_b);
		}
		(Some(ta), None) => {
			let tb = b.bottom_middle + Vec3::Y * b.height;
			mid = mid.with_top_middle(ta * w_a + tb * w_b);
		}
		(None, Some(tb)) => {
			let ta = a.bottom_middle + Vec3::Y * a.height;
			mid = mid.with_top_middle(ta * w_a + tb * w_b);
		}
		(None, None) => {}
	}
	mid
}

fn normalize_xz(v: Vec2) -> Option<Vec2> {
	let len = v.length();
	if len < EPS {
		None
	} else {
		Some(v / len)
	}
}

/// Intersect rays `p_a + t d_a` and `p_b + s d_b` in XZ. Returns `(t, s, point)`.
///
/// Collinear anti-parallel openings (facing each other on one line) use the
/// plan midpoint — a zero-kink special case of the one-kink connector.
fn ray_intersect_xz(p_a: Vec2, d_a: Vec2, p_b: Vec2, d_b: Vec2) -> Option<(f32, f32, Vec2)> {
	let delta = p_b - p_a;
	// det([d_a, -d_b]) = d_b.x*d_a.y - d_a.x*d_b.y
	let det = d_a.y * d_b.x - d_a.x * d_b.y;
	if det.abs() < EPS {
		// Parallel: only succeed when collinear and facing each other.
		let cross = d_a.x * delta.y - d_a.y * delta.x;
		if cross.abs() > EPS {
			return None;
		}
		if d_a.dot(d_b) > -EPS {
			return None;
		}
		let to_b = delta.dot(d_a);
		let to_a = (-delta).dot(d_b);
		if to_b < -EPS || to_a < -EPS {
			return None;
		}
		let point = (p_a + p_b) * 0.5;
		let t = (point - p_a).dot(d_a);
		let s = (point - p_b).dot(d_b);
		return Some((t.max(0.0), s.max(0.0), point));
	}
	let t = (delta.y * d_b.x - delta.x * d_b.y) / det;
	let s = (delta.y * d_a.x - delta.x * d_a.y) / det;
	let point = p_a + d_a * t;
	Some((t, s, point))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn opening_facing(
		center: Vec3,
		half_w: f32,
		half_h: f32,
		orient: Vec2,
	) -> ConnectingHallEndpoint {
		let d = normalize_xz(orient).unwrap();
		let right = Vec3::new(-d.y, 0.0, d.x);
		let up = Vec3::Y;
		let bl = center - right * half_w;
		let br = center + right * half_w;
		let tl = bl + up * (half_h * 2.0);
		let tr = br + up * (half_h * 2.0);
		ConnectingHallEndpoint::new(bl, br, tl, tr, orient)
	}

	#[test]
	fn opposite_openings_meet_on_bisector() {
		// A at x=-4 facing +X; B at x=+4 facing -X → mid at origin.
		let a = opening_facing(Vec3::new(-4.0, 0.0, 0.0), 1.0, 1.0, Vec2::X);
		let b = opening_facing(Vec3::new(4.0, 0.0, 0.0), 1.0, 1.0, -Vec2::X);
		let hall = ConnectingHall::rough_stone(a, b);
		let mid = hall.midpoint();
		assert!(mid.x.abs() < 1e-3, "mid.x={:?}", mid.x);
		assert!(mid.z.abs() < 1e-3, "mid.z={:?}", mid.z);
		assert!(mid.y.abs() < 1e-3);
		assert_eq!(hall.tube().nodes().len(), 3);
		assert!(!hall.tube().floor().pieces().is_empty());
	}

	#[test]
	fn height_is_length_weighted() {
		// Kinked: A at z=-1 facing +Z, B at x=4 facing -X → mid at origin.
		// L_a=1, L_b=4 → h = (4*0 + 1*4)/(1+4) = 0.8
		let a = opening_facing(Vec3::new(0.0, 0.0, -1.0), 1.0, 1.0, Vec2::Y);
		let b = opening_facing(Vec3::new(4.0, 4.0, 0.0), 1.0, 1.0, -Vec2::X);
		let hall = ConnectingHall::rough_stone(a, b);
		let mid = hall.midpoint();
		assert!(mid.x.abs() < 1e-3 && mid.z.abs() < 1e-3, "mid={mid:?}");
		assert!((mid.y - 0.8).abs() < 1e-3, "mid.y={}", mid.y);
	}

	#[test]
	fn kinked_orientations_intersect() {
		let a = opening_facing(Vec3::new(0.0, 0.0, -3.0), 1.0, 1.0, Vec2::Y);
		let b = opening_facing(Vec3::new(3.0, 0.0, 0.0), 1.0, 1.0, -Vec2::X);
		let hall = ConnectingHall::rough_stone(a, b);
		let mid = hall.midpoint();
		assert!((mid.x - 0.0).abs() < 1e-3);
		assert!((mid.z - 0.0).abs() < 1e-3);
		assert_eq!(hall.stations()[1].bottom_middle, mid);
	}

	#[test]
	#[cfg(debug_assertions)]
	#[should_panic(expected = "orientation rays do not meet")]
	fn parallel_orientations_debug_assert() {
		let a = opening_facing(Vec3::new(0.0, 0.0, 0.0), 1.0, 1.0, Vec2::X);
		let b = opening_facing(Vec3::new(0.0, 0.0, 2.0), 1.0, 1.0, Vec2::X);
		let _ = ConnectingHall::rough_stone(a, b);
	}

	#[test]
	#[cfg(not(debug_assertions))]
	fn parallel_orientations_yield_empty_tube() {
		let a = opening_facing(Vec3::new(0.0, 0.0, 0.0), 1.0, 1.0, Vec2::X);
		let b = opening_facing(Vec3::new(0.0, 0.0, 2.0), 1.0, 1.0, Vec2::X);
		let hall = ConnectingHall::rough_stone(a, b);
		assert!(hall.tube().nodes().is_empty() || hall.tube().floor().pieces().is_empty());
	}
}
