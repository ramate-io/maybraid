//! Horizontal shaft-face opening and plan-rim geometry.

use std::ops::Deref;

use bevy_math::{Vec2, Vec3};

use richmond_building_components::panels::PanelStyle;

use crate::connecting::geom::{normalize_xz, EPS};
use crate::openings::MappedOpening;
use crate::paneling::quad_panel::QuadPanel;
use crate::stair_flights::{
	FlightPolyline, FlightStation, SpiralFlight, SpiralFlightFit, StairwellFlight,
	StairwellFlightKind,
};

use super::RUN_IN_M;

/// Plan separation below which the polyline stays a single vertical segment.
const PLAN_KINK_EPS: f32 = 0.15;

/// Horizontal shaft-face opening, typed for [`super::ConnectingStairwell`].
///
/// The quad lies in plan. Lower edge = walk-on. `orientation` is XZ into the well.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairwellOpening(MappedOpening);

impl StairwellOpening {
	pub fn new(mapped: MappedOpening) -> Self {
		Self(mapped)
	}

	pub fn mapped(self) -> MappedOpening {
		self.0
	}

	/// Centroid of the horizontal shaft face.
	pub fn face_center(self) -> Vec3 {
		let (bl, br, tl, tr) = self.endpoint_corners();
		(bl + br + tl + tr) * 0.25
	}

	/// Midpoint of the walk-on (lower) edge.
	pub fn walk_on_mid(self) -> Vec3 {
		let (bl, br, ..) = self.endpoint_corners();
		(bl + br) * 0.5
	}

	/// Length of the walk-on edge (meters).
	pub fn walk_on_width(self) -> f32 {
		let (bl, br, ..) = self.endpoint_corners();
		bl.distance(br)
	}

	/// Half-extent along the walk-on and from walk-on to the far edge.
	pub fn plan_half_extents(self) -> (f32, f32) {
		let (bl, br, tl, tr) = self.endpoint_corners();
		let walk = 0.5 * bl.distance(br);
		let far_mid = (tl + tr) * 0.5;
		let depth = 0.5 * self.walk_on_mid().distance(far_mid);
		(walk.max(EPS), depth.max(EPS))
	}

	/// CCW hole boundary from above: walk-on, \(+\)right, far, \(-\)right.
	pub fn plan_corners_ccw(self) -> [Vec2; 4] {
		let (bl, br, tl, tr) = self.endpoint_corners();
		[plan_xz(bl), plan_xz(br), plan_xz(tr), plan_xz(tl)]
	}

	/// Endpoints of rim edge `i` (CCW, \(i \bmod 4\)).
	pub fn rim_edge(self, i: usize) -> (Vec2, Vec2) {
		let c = self.plan_corners_ccw();
		(c[i % 4], c[(i + 1) % 4])
	}

	/// Nearest point on the opening rim to `p`, with the edge index.
	pub fn nearest_rim(self, p: Vec2) -> Option<(usize, Vec2)> {
		let corners = self.plan_corners_ccw();
		let mut best: Option<(f32, usize, Vec2)> = None;
		for i in 0..4 {
			let a = corners[i];
			let b = corners[(i + 1) % 4];
			let v = b - a;
			let len2 = v.length_squared();
			if len2 < EPS * EPS {
				continue;
			}
			let u = ((p - a).dot(v) / len2).clamp(0.0, 1.0);
			let q = a + v * u;
			let d2 = (p - q).length_squared();
			if best.map(|(bd, ..)| d2 < bd).unwrap_or(true) {
				best = Some((d2, i, q));
			}
		}
		best.map(|(_, i, q)| (i, q))
	}

	/// Plan distance from `p` to the nearest rim point.
	pub fn rim_distance(self, p: Vec2) -> f32 {
		self.nearest_rim(p).map(|(_, q)| (p - q).length()).unwrap_or(f32::MAX)
	}

	/// Thin floor from the walk-on into the shaft (`RUN_IN_M` deep, full walk-on width).
	pub fn run_in_slab(self, style: PanelStyle, thickness: f32) -> QuadPanel {
		let out = normalize_xz(self.orientation).unwrap_or(Vec2::X);
		let walk = self.walk_on_mid();
		let half = (0.5 * self.walk_on_width()).max(EPS);
		let right = Vec3::new(-out.y, 0.0, out.x);
		let inward = Vec3::new(out.x, 0.0, out.y) * RUN_IN_M;
		let a0 = walk - right * half;
		let a1 = walk + right * half;
		QuadPanel::slab(style, a0, a1, a0 + inward, a1 + inward, thickness)
	}

	/// Flight polyline from this face center to `upper` (midpoint only when plan-offset).
	pub fn flight_polyline_to(self, upper: Self) -> FlightPolyline {
		let a = self.face_center();
		let b = upper.face_center();
		let p_a = plan_xz(a);
		let p_b = plan_xz(b);
		let rise = (b.y - a.y).abs().max(EPS);
		let mut stations = vec![FlightStation { center: a, height: rise }];
		if (p_a - p_b).length() > PLAN_KINK_EPS {
			let m_xz = (p_a + p_b) * 0.5;
			let y = 0.5 * (a.y + b.y);
			stations.push(FlightStation { center: Vec3::new(m_xz.x, y, m_xz.y), height: rise });
		}
		stations.push(FlightStation { center: b, height: rise });
		FlightPolyline { stations }
	}

	/// Spiral fit inputs for a well from this lower face to `upper`.
	pub fn spiral_fit(self, upper: Self) -> SpiralFlightFit {
		let (lower_hw, lower_hd) = self.plan_half_extents();
		let (upper_hw, upper_hd) = upper.plan_half_extents();
		SpiralFlightFit {
			lower_center: self.face_center(),
			upper_center: upper.face_center(),
			lower_walk_on: self.walk_on_mid(),
			upper_walk_on: upper.walk_on_mid(),
			lower_out: self.orientation,
			lower_half_width: lower_hw,
			lower_half_depth: lower_hd,
			upper_half_width: upper_hw,
			upper_half_depth: upper_hd,
		}
	}

	/// Fitted spiral along [`Self::flight_polyline_to`].
	pub fn spiral_flight_to(self, upper: Self) -> SpiralFlight {
		let polyline = self.flight_polyline_to(upper);
		SpiralFlight::fit(polyline, self.spiral_fit(upper))
	}

	/// Fitted flight of `kind` along [`Self::flight_polyline_to`].
	pub fn flight_to(
		self,
		upper: Self,
		kind: StairwellFlightKind,
		style: PanelStyle,
		slab_thickness: f32,
	) -> StairwellFlight {
		StairwellFlight::fit(
			kind,
			self.flight_polyline_to(upper),
			self.spiral_fit(upper),
			style,
			slab_thickness,
		)
	}
}

impl From<MappedOpening> for StairwellOpening {
	fn from(mapped: MappedOpening) -> Self {
		Self(mapped)
	}
}

impl Deref for StairwellOpening {
	type Target = MappedOpening;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

pub(crate) fn plan_xz(p: Vec3) -> Vec2 {
	Vec2::new(p.x, p.z)
}

pub(crate) fn at_y(p: Vec2, y: f32) -> Vec3 {
	Vec3::new(p.x, y, p.y)
}
