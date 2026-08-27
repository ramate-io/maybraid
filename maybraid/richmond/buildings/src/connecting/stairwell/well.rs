//! Exclusive orthogonal well: one box, two cardinal doors.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};

use richmond_building_components::panels::PanelStyle;

use crate::paneling::quad_panel::QuadPanel;

use super::opening::StairwellOpening;
use super::RUN_IN_M;

pub const TREAD_FILL_DEFAULT: f32 = 0.4;
pub const TREAD_FILL_MIN: f32 = 0.2;
pub const TREAD_FILL_MAX: f32 = 0.95;
const TREAD_WIDTH_MIN_M: f32 = 0.35;

const EPS: f32 = 1e-4;

/// Cardinal side of the well's plan rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WellSide {
	NegX,
	PosX,
	NegZ,
	PosZ,
}

impl WellSide {
	/// Unit XZ pointing from the plan center toward this side.
	pub fn into_xz(self) -> Vec2 {
		match self {
			Self::NegX => -Vec2::X,
			Self::PosX => Vec2::X,
			Self::NegZ => -Vec2::Y,
			Self::PosZ => Vec2::Y,
		}
	}

	pub fn nearest(bounds: Aabb3d, p: Vec3) -> Self {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let d = [
			((p.x - min.x).abs(), Self::NegX),
			((p.x - max.x).abs(), Self::PosX),
			((p.z - min.z).abs(), Self::NegZ),
			((p.z - max.z).abs(), Self::PosZ),
		];
		d.into_iter()
			.min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
			.map(|(_, s)| s)
			.unwrap_or(Self::NegZ)
	}
}

/// Exclusive axis-aligned well. Same plan on the bottom and top faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WellAabb {
	pub bounds: Aabb3d,
	pub walk_on: WellSide,
	pub walk_off: WellSide,
	pub tread_fill: f32,
}

impl WellAabb {
	pub fn from_plan(
		min: Vec3,
		max: Vec3,
		walk_on: WellSide,
		walk_off: WellSide,
		tread_fill: f32,
	) -> Self {
		Self {
			bounds: Aabb3d::from_min_max(min.min(max), min.max(max)),
			walk_on,
			walk_off,
			tread_fill: clamp_tread_fill(tread_fill),
		}
	}

	/// Union of the two shaft faces, doors snapped to the nearest box sides.
	pub fn allocate(lower: StairwellOpening, upper: StairwellOpening, tread_fill: f32) -> Self {
		let mut min = lower.face_center();
		let mut max = min;
		for p in lower.corners().into_iter().chain(upper.corners()) {
			min = min.min(p);
			max = max.max(p);
		}
		if (max.y - min.y).abs() < EPS {
			max.y = min.y;
		}
		let bounds = Aabb3d::from_min_max(min, max);
		Self {
			bounds,
			walk_on: WellSide::nearest(bounds, lower.walk_on_mid()),
			walk_off: WellSide::nearest(bounds, upper.walk_on_mid()),
			tread_fill: clamp_tread_fill(tread_fill),
		}
	}

	pub fn min(self) -> Vec3 {
		Vec3::from(self.bounds.min)
	}

	pub fn max(self) -> Vec3 {
		Vec3::from(self.bounds.max)
	}

	pub fn center_xz(self) -> Vec2 {
		let min = self.min();
		let max = self.max();
		Vec2::new(0.5 * (min.x + max.x), 0.5 * (min.z + max.z))
	}

	pub fn half_x(self) -> f32 {
		0.5 * (self.max().x - self.min().x).max(EPS)
	}

	pub fn half_z(self) -> f32 {
		0.5 * (self.max().z - self.min().z).max(EPS)
	}

	pub fn half_min(self) -> f32 {
		self.half_x().min(self.half_z())
	}

	pub fn bottom_y(self) -> f32 {
		self.min().y
	}

	pub fn top_y(self) -> f32 {
		self.max().y
	}

	pub fn rise(self) -> f32 {
		(self.top_y() - self.bottom_y()).abs()
	}

	pub fn contains_xz(self, x: f32, z: f32) -> bool {
		let min = self.min();
		let max = self.max();
		x >= min.x - 0.2 && x <= max.x + 0.2 && z >= min.z - 0.2 && z <= max.z + 0.2
	}

	/// Tread width from [`Self::tread_fill`] of the tighter half-extent.
	pub fn tread_width(self) -> f32 {
		let half = self.half_min();
		(half * self.tread_fill)
			.clamp(TREAD_WIDTH_MIN_M.min(half * TREAD_FILL_MAX), half * TREAD_FILL_MAX)
	}

	pub fn side_mid(self, side: WellSide, y: f32) -> Vec3 {
		let c = self.center_xz();
		let min = self.min();
		let max = self.max();
		match side {
			WellSide::NegX => Vec3::new(min.x, y, c.y),
			WellSide::PosX => Vec3::new(max.x, y, c.y),
			WellSide::NegZ => Vec3::new(c.x, y, min.z),
			WellSide::PosZ => Vec3::new(c.x, y, max.z),
		}
	}

	/// Axis-aligned strip along `side`, `depth` into the well, `half_along` wide.
	pub fn side_strip(self, side: WellSide, y: f32, depth: f32, half_along: f32) -> [Vec3; 4] {
		let inward = -side.into_xz();
		let along = Vec2::new(-inward.y, inward.x);
		let outer = self.side_mid(side, y);
		let o = Vec2::new(outer.x, outer.z);
		let half = half_along.max(EPS);
		let a0 = o - along * half;
		let a1 = o + along * half;
		let b0 = a0 + inward * depth.max(EPS);
		let b1 = a1 + inward * depth.max(EPS);
		[
			Vec3::new(a0.x, y, a0.y),
			Vec3::new(a1.x, y, a1.y),
			Vec3::new(b0.x, y, b0.y),
			Vec3::new(b1.x, y, b1.y),
		]
	}

	/// Run-in: full walk-on width, [`RUN_IN_M`] into the box.
	pub fn run_in_slab(self, style: PanelStyle, thickness: f32) -> QuadPanel {
		let along = match self.walk_on {
			WellSide::NegX | WellSide::PosX => self.half_z(),
			WellSide::NegZ | WellSide::PosZ => self.half_x(),
		};
		let [a0, a1, b0, b1] = self.side_strip(self.walk_on, self.bottom_y(), RUN_IN_M, along);
		QuadPanel::slab(style, a0, a1, b0, b1, thickness)
	}

	/// Walk-off landing: full exclusive-box span on the door, fixed inward depth.
	///
	/// A strip, not a hull to the last tread — that lid ate headroom on a
	/// quarter-turn. Depth is capped at half the tighter half-extent so the
	/// spiral hole stays open.
	pub fn walk_off_landing_strip(
		self,
		style: PanelStyle,
		thickness: f32,
		depth: f32,
	) -> QuadPanel {
		let along = match self.walk_off {
			WellSide::NegX | WellSide::PosX => self.half_z(),
			WellSide::NegZ | WellSide::PosZ => self.half_x(),
		};
		let cap = self.half_min() * 0.55;
		let d = depth.max(MIN_LANDING_ALONG).min(cap).max(MIN_LANDING_ALONG);
		let [a0, a1, b0, b1] = self.side_strip(self.walk_off, self.top_y(), d, along);
		QuadPanel::slab(style, a0, a1, b0, b1, thickness)
	}

	/// Interior midpoint of a walk-off strip of `landing_depth`.
	pub fn back_point_xz(self, landing_depth: f32) -> Vec2 {
		let outer = self.side_mid(self.walk_off, self.top_y());
		let inward = -self.walk_off.into_xz();
		Vec2::new(outer.x, outer.z) + inward * landing_depth.max(EPS)
	}
}

const MIN_LANDING_ALONG: f32 = 0.12;

pub fn clamp_tread_fill(fill: f32) -> f32 {
	if !fill.is_finite() {
		TREAD_FILL_DEFAULT
	} else {
		fill.clamp(TREAD_FILL_MIN, TREAD_FILL_MAX)
	}
}
