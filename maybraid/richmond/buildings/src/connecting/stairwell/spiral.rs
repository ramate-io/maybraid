//! Circular flight inside an exclusive [`WellAabb`].
//!
//! Inscribe a circle so the outer rail stays in the box. First tread at the
//! walk-on azimuth. After the last tread exists, grow an axis-aligned walk-off
//! landing until it covers that leading. Extra turns only when going would fall
//! under [`MIN_GOING`].

use std::f32::consts::TAU;

use bevy_math::{Vec2, Vec3};
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::placed::Placement;
use richmond_building_components::stairs::{Stair, StairNode, StraightStair};

use crate::paneling::quad_panel::QuadPanel;

use super::tread::TreadEnd;
use super::well::WellAabb;

/// Smallest walkable going (meters). Extra turns exist only to stay at or above this.
pub(crate) const MIN_GOING: f32 = 0.25;
const MIN_RADIUS: f32 = 0.2;
const MIN_LANDING: f32 = 0.12;

/// Circular nodes + the walk-off landing (AABB strip, not a sheared pad).
pub(crate) fn fit(
	well: &WellAabb,
	style: PanelStyle,
	thickness: f32,
) -> (Vec<StairNode>, Option<QuadPanel>) {
	let rise = well.rise().max(StraightStair::DEFAULT_TREAD_HEIGHT);
	let width = well.tread_width();
	let radius = (well.half_min() - MIN_LANDING - 0.5 * width).max(MIN_RADIUS);
	let n = (rise / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0) as u32;
	let turns = spiral_turns(well, radius, n);
	let going = (turns * TAU * radius) / n as f32;
	let center = well.center_xz();
	let start_yaw = yaw_toward(well.walk_on.into_xz());
	let stairs =
		circular_nodes(center, well.bottom_y(), start_yaw, radius, width, going, rise, n, turns);
	let landing = TreadEnd::from_last_straight(&stairs).and_then(|end| {
		let stand = end.width.max(end.going);
		well.walk_off_landing_covering(style, thickness, &landing_cover_points(well, end), stand)
	});
	(stairs, landing)
}

/// Last-tread plan plus a standable square on the walk-off (AABB union).
fn landing_cover_points(well: &WellAabb, end: TreadEnd) -> Vec<Vec2> {
	let mut pts = end.plan_quad().to_vec();
	let stand = end.width.max(end.going);
	let mid = well.side_mid(well.walk_off, well.top_y());
	let door = well.walk_off.into_xz();
	let along = Vec2::new(-door.y, door.x);
	let inward = -door;
	let o = Vec2::new(mid.x, mid.z);
	let half = 0.5 * stand;
	pts.extend([
		o - along * half,
		o + along * half,
		o - along * half + inward * stand,
		o + along * half + inward * stand,
	]);
	pts
}

fn spiral_turns(well: &WellAabb, radius: f32, n: u32) -> f32 {
	let start = angle_of(well.walk_on.into_xz());
	let back = angle_of(well.walk_off.into_xz());
	let mut sweep = wrap_ccw(back - start);
	if sweep < 0.2 * TAU {
		sweep += TAU;
	}
	let mut turns = sweep / TAU;
	let r = radius.max(1e-4);
	while (turns * TAU * r) / n as f32 + 1e-4 < MIN_GOING {
		turns += 1.0;
	}
	turns
}

fn circular_nodes(
	center: Vec2,
	base_y: f32,
	start_yaw: f32,
	radius: f32,
	width: f32,
	going: f32,
	rise: f32,
	n: u32,
	turns: f32,
) -> Vec<StairNode> {
	if n == 0 {
		return Vec::new();
	}
	let yaw_step = turns.max(1e-4) * TAU / n as f32;
	let rise_step = rise / n as f32;
	let (ys, yc) = start_yaw.sin_cos();
	let rotate = |lx: f32, lz: f32| Vec2::new(yc * lx + ys * lz, -ys * lx + yc * lz);

	(0..n)
		.map(|i| {
			let theta = i as f32 * yaw_step;
			let (s, c) = theta.sin_cos();
			let p = center + rotate(c * radius, -s * radius);
			let travel_yaw = start_yaw + theta + std::f32::consts::FRAC_PI_2;
			let y = base_y + i as f32 * rise_step;
			StairNode::rough_stone(
				Stair::Straight(
					StraightStair::single(width, going.max(1e-4), rise_step)
						.with_flush_start(false),
				),
				Placement::new(Vec3::new(p.x, y, p.y), travel_yaw),
			)
		})
		.collect()
}

fn yaw_toward(dir: Vec2) -> f32 {
	let d = if dir.length_squared() < 1e-8 { Vec2::X } else { dir.normalize() };
	(-d.y).atan2(d.x)
}

fn angle_of(dir: Vec2) -> f32 {
	if dir.length_squared() < 1e-8 {
		0.0
	} else {
		dir.y.atan2(dir.x)
	}
}

fn wrap_ccw(delta: f32) -> f32 {
	let mut d = delta;
	while d <= 0.0 {
		d += TAU;
	}
	d
}
