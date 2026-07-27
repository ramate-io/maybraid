//! Outer circular wall ring with door/window openings.
//!
//! Ring is authored in 15° kit pieces:
//! - **Door** at \(0^\circ\): omit two full-height 15° arcs; place 15° headers at
//!   \(0.7 \times\) storey height (lintel).
//! - **Window** at each \(90^\circ\): omit two full-height 15° arcs; place bottom and
//!   top 15° headers (sill at floor, lintel at \(0.7 \times\) storey height).

use bevy_math::Vec3;
use richmond_building_components::partitions::Wall;
use richmond_building_components::Placed;

const SEG_DEG: f32 = 15.0;
/// Half-width of each opening in degrees (two 15° segments → 30° total).
const OPEN_HALF_DEG: f32 = 15.0;
/// Lintel / top-header baseline as a fraction of storey height.
const HEADER_Y_FRAC: f32 = 0.7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpeningKind {
	Door,
	Window,
}

/// Door at \(0^\circ\), windows at \(90^\circ / 180^\circ / 270^\circ\).
const OPENINGS: [(f32, OpeningKind); 4] = [
	(0.0, OpeningKind::Door),
	(90.0, OpeningKind::Window),
	(180.0, OpeningKind::Window),
	(270.0, OpeningKind::Window),
];

fn norm_deg(deg: f32) -> f32 {
	let mut d = deg % 360.0;
	if d < 0.0 {
		d += 360.0;
	}
	d
}

/// Circular outer wall with door/window breaks around `center_xz`.
pub fn outer_ring_with_openings(
	center_xz: Vec3,
	radius: f32,
	storey_height: f32,
) -> Vec<Placed<Wall>> {
	let radius = radius.max(1e-4);
	let storey_height = storey_height.max(1e-4);
	let ring_scale = Vec3::new(radius, storey_height, radius);
	let lintel = center_xz + Vec3::Y * (HEADER_Y_FRAC * storey_height);
	let mut walls = Vec::new();

	for (center_deg, kind) in OPENINGS {
		let open_start = center_deg - OPEN_HALF_DEG;
		for i in 0..2 {
			let seg_start = norm_deg(open_start + i as f32 * SEG_DEG);
			let yaw = seg_start.to_radians();
			match kind {
				OpeningKind::Door => {
					walls.push(
						Placed::new(Wall::header_arc(SEG_DEG), lintel, yaw).with_scale(ring_scale),
					);
				}
				OpeningKind::Window => {
					walls.push(
						Placed::new(Wall::header_arc(SEG_DEG), center_xz, yaw).with_scale(ring_scale),
					);
					walls.push(
						Placed::new(Wall::header_arc(SEG_DEG), lintel, yaw).with_scale(ring_scale),
					);
				}
			}
		}
	}

	for i in 0..OPENINGS.len() {
		let (c0, _) = OPENINGS[i];
		let (c1, _) = OPENINGS[(i + 1) % OPENINGS.len()];
		let solid_start = norm_deg(c0 + OPEN_HALF_DEG);
		let solid_end = norm_deg(c1 - OPEN_HALF_DEG);
		let sweep = if solid_end >= solid_start - 1e-3 {
			solid_end - solid_start
		} else {
			solid_end + 360.0 - solid_start
		};
		if sweep > 1e-2 {
			walls.push(
				Placed::new(Wall::arc(sweep), center_xz, solid_start.to_radians())
					.with_scale(ring_scale),
			);
		}
	}

	walls
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ring_has_solids_and_headers() -> anyhow::Result<()> {
		let walls = outer_ring_with_openings(Vec3::ZERO, 4.0, 3.0);
		let headers = walls
			.iter()
			.filter(|w| matches!(w.geom, Wall::HeaderArc(_)))
			.count();
		let solids = walls
			.iter()
			.filter(|w| matches!(w.geom, Wall::Arc(_)))
			.count();
		// Door: 2 top headers. Windows ×3: 2 segs × (bottom+top) = 12. Total 14 headers.
		assert_eq!(headers, 14);
		// Four solid 60° runs between openings.
		assert_eq!(solids, 4);
		Ok(())
	}
}
