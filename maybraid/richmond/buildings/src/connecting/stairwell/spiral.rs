//! Circular flight inside an exclusive [`WellAabb`].
//!
//! Inscribe a circle so the outer rail stays in the box. First tread at the
//! walk-on azimuth, last at the walk-off. The walk-off landing is a door strip
//! authored first; the last leading arrives on that strip. Extra turns only
//! when going would fall under [`MIN_GOING`] and rise-per-turn still has
//! [`MIN_HEADROOM`].

use std::f32::consts::TAU;

use bevy_math::{Vec2, Vec3};
use richmond_building_components::panels::PanelStyle;
use richmond_building_components::placed::Placement;
use richmond_building_components::stairs::{Stair, StairNode, StraightStair};

use crate::paneling::quad_panel::QuadPanel;

use super::well::WellAabb;

/// Smallest walkable going (meters). Extra turns exist only to stay at or above this
/// when [`MIN_HEADROOM`] still holds.
pub(crate) const MIN_GOING: f32 = 0.25;
/// Smallest rise per revolution (meters). A short well keeps one lap and accepts
/// going below [`MIN_GOING`] rather than stacking helices.
pub(crate) const MIN_HEADROOM: f32 = 2.0;
const MIN_RADIUS: f32 = 0.2;
const MIN_LANDING: f32 = 0.12;

/// Circular nodes + the walk-off landing (door strip, not a sheared pad).
pub(crate) fn fit(
	well: &WellAabb,
	style: PanelStyle,
	thickness: f32,
) -> (Vec<StairNode>, Option<QuadPanel>) {
	let rise = well.rise().max(StraightStair::DEFAULT_TREAD_HEIGHT);
	let width = well.tread_width();
	let radius = (well.half_min() - MIN_LANDING - 0.5 * width).max(MIN_RADIUS);
	let n = (rise / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0) as u32;
	let turns = spiral_turns(well, radius, n, rise);
	let intervals = n.saturating_sub(1).max(1);
	let going = (turns * TAU * radius) / intervals as f32;
	let center = well.center_xz();
	let start_yaw = yaw_toward(well.walk_on.into_xz());
	let stairs =
		circular_nodes(center, well.bottom_y(), start_yaw, radius, width, going, rise, n, turns);
	// Door to last inner rail: reserved rim + tread span. Capped in the strip.
	let depth = width + MIN_LANDING;
	let landing = Some(well.walk_off_landing_strip(style, thickness, depth));
	(stairs, landing)
}

fn spiral_turns(well: &WellAabb, radius: f32, n: u32, rise: f32) -> f32 {
	// Same yaw convention as [`circular_nodes`] so walk-off is the last azimuth.
	let start = yaw_toward(well.walk_on.into_xz());
	let end = yaw_toward(well.walk_off.into_xz());
	let mut sweep = wrap_ccw(end - start);
	if sweep < 0.2 * TAU {
		sweep += TAU;
	}
	let mut turns = sweep / TAU;
	let r = radius.max(1e-4);
	let intervals = n.saturating_sub(1).max(1) as f32;
	while (turns * TAU * r) / intervals + 1e-4 < MIN_GOING {
		let next = turns + 1.0;
		if rise / next + 1e-4 < MIN_HEADROOM {
			break;
		}
		turns = next;
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
	let sweep = turns.max(1e-4) * TAU;
	let rise_step = rise / n as f32;
	let (ys, yc) = start_yaw.sin_cos();
	let rotate = |lx: f32, lz: f32| Vec2::new(yc * lx + ys * lz, -ys * lx + yc * lz);

	(0..n)
		.map(|i| {
			let t = if n == 1 { 1.0 } else { i as f32 / (n - 1) as f32 };
			let theta = t * sweep;
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

fn wrap_ccw(delta: f32) -> f32 {
	let mut d = delta;
	while d <= 0.0 {
		d += TAU;
	}
	d
}
