//! Straight runs and quarter-cylinder corner loci for a rounded rectangle.

use bevy_math::{Vec2, Vec3};
use std::f32::consts::FRAC_PI_2;

use crate::shells::ortho::{OrthoSide, PlanRect, WallEdge};

use super::RoundedRectFloorParams;

/// Corner of a rounded rectangle (SE, NE, NW, SW in CCW order from south-east).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoundedRectCorner {
	/// +X, −Z
	SouthEast = 0,
	/// +X, +Z
	NorthEast = 1,
	/// −X, +Z
	NorthWest = 2,
	/// −X, −Z
	SouthWest = 3,
}

impl RoundedRectCorner {
	pub fn all() -> [Self; 4] {
		[Self::SouthEast, Self::NorthEast, Self::NorthWest, Self::SouthWest]
	}

	pub fn index(self) -> usize {
		self as usize
	}

	/// Arc center in plan.
	pub fn center(self, plan: PlanRect, radius: f32) -> Vec3 {
		let r = radius.max(0.0);
		match self {
			Self::SouthEast => Vec3::new(
				plan.center.x + plan.half_x - r,
				plan.y,
				plan.center.z - plan.half_z + r,
			),
			Self::NorthEast => Vec3::new(
				plan.center.x + plan.half_x - r,
				plan.y,
				plan.center.z + plan.half_z - r,
			),
			Self::NorthWest => Vec3::new(
				plan.center.x - plan.half_x + r,
				plan.y,
				plan.center.z + plan.half_z - r,
			),
			Self::SouthWest => Vec3::new(
				plan.center.x - plan.half_x + r,
				plan.y,
				plan.center.z - plan.half_z + r,
			),
		}
	}

	/// Start angle (from +X toward +Z) of the exterior quarter arc, walking CCW.
	pub fn start_angle(self) -> f32 {
		match self {
			Self::SouthEast => -FRAC_PI_2, // −Y plan / −Z: from south run end toward east
			Self::NorthEast => 0.0,
			Self::NorthWest => FRAC_PI_2,
			Self::SouthWest => std::f32::consts::PI,
		}
	}
}

/// Resolved wall geometry for one storey.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RoundedRectGeom {
	pub plan: PlanRect,
	pub radius: f32,
	pub height: f32,
	pub segments: u32,
	/// Straight edges in OrthoSide order (South, East, North, West).
	pub straights: [WallEdge; 4],
	/// Bottom-rail samples for each corner (including both tangent endpoints).
	pub corner_bottom: [Vec<Vec3>; 4],
	pub corner_top: [Vec<Vec3>; 4],
}

impl RoundedRectFloorParams {
	pub(super) fn resolve_geometry(&self, plan: PlanRect, radius: f32) -> RoundedRectGeom {
		let height = self.storey_height.max(1e-4);
		let segments = self.corner_segments.max(1);
		let r = radius.max(0.0);

		let straights = if r < 1e-4 {
			// Degenerate to full-side edges.
			[
				WallEdge::new(plan.sw(), plan.se(), height, OrthoSide::South.orientation()),
				WallEdge::new(plan.se(), plan.ne(), height, OrthoSide::East.orientation()),
				WallEdge::new(plan.ne(), plan.nw(), height, OrthoSide::North.orientation()),
				WallEdge::new(plan.nw(), plan.sw(), height, OrthoSide::West.orientation()),
			]
		} else {
			[
				WallEdge::new(
					Vec3::new(plan.center.x - plan.half_x + r, plan.y, plan.center.z - plan.half_z),
					Vec3::new(plan.center.x + plan.half_x - r, plan.y, plan.center.z - plan.half_z),
					height,
					OrthoSide::South.orientation(),
				),
				WallEdge::new(
					Vec3::new(plan.center.x + plan.half_x, plan.y, plan.center.z - plan.half_z + r),
					Vec3::new(plan.center.x + plan.half_x, plan.y, plan.center.z + plan.half_z - r),
					height,
					OrthoSide::East.orientation(),
				),
				WallEdge::new(
					Vec3::new(plan.center.x + plan.half_x - r, plan.y, plan.center.z + plan.half_z),
					Vec3::new(plan.center.x - plan.half_x + r, plan.y, plan.center.z + plan.half_z),
					height,
					OrthoSide::North.orientation(),
				),
				WallEdge::new(
					Vec3::new(plan.center.x - plan.half_x, plan.y, plan.center.z + plan.half_z - r),
					Vec3::new(plan.center.x - plan.half_x, plan.y, plan.center.z - plan.half_z + r),
					height,
					OrthoSide::West.orientation(),
				),
			]
		};

		let mut corner_bottom = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
		let mut corner_top = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
		for corner in RoundedRectCorner::all() {
			let i = corner.index();
			let (bot, top) = sample_corner_rails(plan, r, height, segments, corner);
			corner_bottom[i] = bot;
			corner_top[i] = top;
		}

		RoundedRectGeom {
			plan,
			radius: r,
			height,
			segments,
			straights,
			corner_bottom,
			corner_top,
		}
	}
}

fn sample_corner_rails(
	plan: PlanRect,
	radius: f32,
	height: f32,
	segments: u32,
	corner: RoundedRectCorner,
) -> (Vec<Vec3>, Vec<Vec3>) {
	if radius < 1e-4 {
		return (Vec::new(), Vec::new());
	}
	let c = corner.center(plan, radius);
	let start = corner.start_angle();
	let n = segments.max(1);
	let mut bot = Vec::with_capacity(n as usize + 1);
	let mut top = Vec::with_capacity(n as usize + 1);
	for i in 0..=n {
		let t = i as f32 / n as f32;
		let ang = start + t * FRAC_PI_2;
		let dir = Vec2::new(ang.cos(), ang.sin());
		let p = Vec3::new(c.x + dir.x * radius, plan.y, c.z + dir.y * radius);
		bot.push(p);
		top.push(p + Vec3::Y * height);
	}
	(bot, top)
}

/// Core plan rectangle inset by the corner radius (straight fill between corners).
pub(super) fn core_plan(plan: PlanRect, radius: f32) -> PlanRect {
	let r = radius.max(0.0);
	PlanRect::new(
		plan.center,
		(plan.full_x() - 2.0 * r).max(1e-3),
		(plan.full_z() - 2.0 * r).max(1e-3),
	)
}
