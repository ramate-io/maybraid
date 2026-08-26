//! Last-tread contact for a landing (flight-agnostic).

use bevy_math::Vec2;
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::stairs::{Stair, StairNode};

use crate::paneling::quad_panel::QuadPanel;
use crate::stair_flights::geom::{
	is_yaw_joint, level_rect, normalize_xz, travel_xz, xz, JointBudget, EPS,
};

/// Leading edge and travel of the last tread — enough to author a landing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreadEnd {
	pub leading_outer: Vec2,
	pub leading_inner: Vec2,
	pub travel: Vec2,
}

impl TreadEnd {
	/// Last tread of a linear [`Stair::Straight`] node.
	///
	/// Outer is the CCW perpendicular of travel (same convention as the circular
	/// flight). Swap with [`Self::prefer_outer_near`] when a rim is known.
	pub fn from_straight(node: &StairNode) -> Self {
		let (width, going, n) = match &node.geometry {
			Stair::Straight(g) => (g.width, g.going_per_tread(), g.tread_count().max(1)),
		};
		let travel = travel_xz(node.placement.yaw);
		let radial = Vec2::new(-travel.y, travel.x);
		let origin = xz(node.placement.translation);
		let last = origin + travel * ((n - 1) as f32 * going);
		let lead = last + travel * (0.5 * going);
		let half_w = 0.5 * width;
		Self {
			leading_outer: lead + radial * half_w,
			leading_inner: lead - radial * half_w,
			travel,
		}
	}

	pub fn from_last_straight(nodes: &[StairNode]) -> Option<Self> {
		nodes.last().map(Self::from_straight)
	}

	/// Prefer the leading endpoint nearer `p` as the outer (rim) corner.
	pub fn prefer_outer_near(mut self, p: Vec2) -> Self {
		if (self.leading_inner - p).length_squared() < (self.leading_outer - p).length_squared() {
			std::mem::swap(&mut self.leading_outer, &mut self.leading_inner);
		}
		self
	}

	/// Vector along the leading edge, outer → inner.
	pub fn leading(self) -> Vec2 {
		self.leading_inner - self.leading_outer
	}

	/// Thin slab starting on this leading edge, extruded along `along` (travel half-plane).
	pub fn landing_toward(
		self,
		along: Vec2,
		y: f32,
		style: PanelStyle,
		thickness: f32,
		length: f32,
		min_length: f32,
	) -> Option<QuadPanel> {
		let along = {
			let n = along.length();
			if n < EPS {
				return None;
			}
			let dir = along / n;
			if dir.dot(self.travel) >= 0.0 {
				dir
			} else {
				-dir
			}
		};
		self.landing_along(along, y, style, thickness, length, min_length)
	}

	/// Thin slab starting on this leading edge, extruded exactly along `along`.
	///
	/// Use at yaw jumps and when routing to a walk-on that is not in the travel
	/// half-plane. `along` is normalized by this method.
	pub fn landing_along(
		self,
		along: Vec2,
		y: f32,
		style: PanelStyle,
		thickness: f32,
		length: f32,
		min_length: f32,
	) -> Option<QuadPanel> {
		let n = along.length();
		if n < EPS || length < min_length {
			return None;
		}
		let along = along / n;
		let lead = self.leading();
		if lead.length_squared() < EPS * EPS {
			return None;
		}
		let a0 = self.leading_outer;
		let b0 = a0 + lead;
		Some(level_rect(style, a0, a0 + along * length, b0, b0 + along * length, y, thickness))
	}

	/// Midpoint of the leading edge in XZ.
	pub fn leading_mid(self) -> Vec2 {
		(self.leading_outer + self.leading_inner) * 0.5
	}

	/// Rest pad flush with this leading: incoming × outgoing at a yaw, else incoming strip.
	pub(crate) fn pad(
		self,
		outgoing: Option<Vec2>,
		budget: JointBudget,
		tread_width: f32,
		y: f32,
		style: PanelStyle,
		thickness: f32,
	) -> Option<QuadPanel> {
		let incoming = normalize_xz(self.travel)?;
		let along = budget.along.max(1e-4);
		let joint = self.leading_mid() + incoming * along;
		let half = 0.5 * tread_width.max(1e-4);
		if let Some(out) = outgoing.and_then(normalize_xz) {
			if is_yaw_joint(incoming, out) {
				let far = budget.far.max(1e-4);
				let pt = |u: f32, v: f32| joint + incoming * u + out * v;
				// U rest: stop at the inner edge so the return walk off it, not along the side.
				let past = if budget.u_turn { 0.0 } else { half };
				return Some(level_rect(
					style,
					pt(-along, -half),
					pt(past, -half),
					pt(-along, far),
					pt(past, far),
					y,
					thickness,
				));
			}
		}
		self.landing_along(incoming, y, style, thickness, along, 1e-4)
	}
}
