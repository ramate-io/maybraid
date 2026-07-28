//! Parameterized arc wall with portal (door/window) openings.
//!
//! \(t \in [0, 1]\) runs along the wall’s covered sweep ([`ArcWallParams::arc_degrees`]),
//! not necessarily a full circle.
//!
//! Construction:
//! 1. **Must-assign** — best-fit a portal into each required region.
//! 2. **Can-assign** — the unit arc minus the union of must-assign and must-not
//!    regions (plus footprints of portals already placed).
//! 3. **Optional** — sample portal noise for how many optional portals to attempt
//!    in \([min, max]\) and where to place them in can-assign space.

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::partitions::Wall;
use richmond_building_components::Placed;

/// Kit segment size (degrees) and portal width (two segments → 30°).
const SEG_DEG: f32 = 15.0;
const PORTAL_SEGS: u32 = 2;
/// Half-width of each portal in degrees (two 15° segments → 30° total).
const OPEN_HALF_DEG: f32 = SEG_DEG * (PORTAL_SEGS as f32) * 0.5;
/// Portal width in degrees.
const PORTAL_WIDTH_DEG: f32 = OPEN_HALF_DEG * 2.0;
/// Lintel / top-header baseline as a fraction of storey height.
const HEADER_Y_FRAC: f32 = 0.7;

/// Opening cut into an arc wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Portal {
	Door,
	Window,
}

/// Inclusive–exclusive interval on the unit arc \(t \in [0, 1)\).
///
/// When `start == end` the region is a **point** locus (preferred / forbidden \(t\)).
/// When `start > end` the interval wraps across \(0\) (closed arcs only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcRegion {
	pub start: f32,
	pub end: f32,
}

impl ArcRegion {
	/// Point region at normalized \(t\).
	pub fn point(t: f32) -> Self {
		let t = norm_t(t);
		Self { start: t, end: t }
	}

	/// Half-open span `[start, end)` on the unit arc (wraps if `start > end`).
	pub fn span(start: f32, end: f32) -> Self {
		Self {
			start: norm_t(start),
			end: norm_t(end),
		}
	}

	pub fn is_point(self) -> bool {
		(self.start - self.end).abs() < 1e-6
	}

	/// Arc length in \(t\)-units (points have length 0).
	pub fn length(self) -> f32 {
		if self.is_point() {
			return 0.0;
		}
		if self.end >= self.start {
			self.end - self.start
		} else {
			self.end + 1.0 - self.start
		}
	}

	pub fn midpoint(self) -> f32 {
		if self.is_point() {
			return self.start;
		}
		norm_t(self.start + 0.5 * self.length())
	}

	/// Whether normalized \(t\) lies in this region (points match within epsilon).
	pub fn contains_t(self, t: f32) -> bool {
		let t = norm_t(t);
		if self.is_point() {
			return circular_dist(t, self.start) < 1e-5;
		}
		if self.end >= self.start {
			t >= self.start && t < self.end
		} else {
			t >= self.start || t < self.end
		}
	}
}

/// Required portal: best-fit into `region`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MustAssignPortal {
	pub region: ArcRegion,
	pub portal: Portal,
}

impl MustAssignPortal {
	pub fn at(t: f32, portal: Portal) -> Self {
		Self {
			region: ArcRegion::point(t),
			portal,
		}
	}
}

/// Parameters for [`ArcWall::new`].
#[derive(Debug, Clone)]
pub struct ArcWallParams {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	/// Degrees of arc this wall covers (\((0, 360]\); \(360\) is a closed ring).
	pub arc_degrees: f32,
	/// Regions that **must** receive a portal (best-fit). \(t\) is along this arc.
	pub must_assign: Vec<MustAssignPortal>,
	/// Regions that **must not** receive a portal.
	pub must_not_assign: Vec<ArcRegion>,
	/// Noise used for optional portal count and placement.
	pub portal_noise: NoiseParams,
	/// Inclusive \((min, max)\) optional portals to attempt in can-assign space.
	pub optional_portals: (u32, u32),
}

/// Portal assigned on the arc (center \(t\), kind).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssignedPortal {
	pub t: f32,
	pub portal: Portal,
}

/// Arc-shaped wall with door/window openings.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcWall {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	pub arc_degrees: f32,
	pub portals: Vec<AssignedPortal>,
	pub walls: Vec<Placed<Wall>>,
}

impl ArcWall {
	/// Assign must portals, then noise-sample optional portals in can-assign regions.
	pub fn new(params: ArcWallParams) -> Self {
		let radius = params.radius.max(1e-4);
		let storey_height = params.storey_height.max(1e-4);
		let arc_degrees = params.arc_degrees.clamp(SEG_DEG, 360.0);
		let noise = NoiseConfig::new(params.portal_noise);
		let closed = is_closed(arc_degrees);

		let mut portals = Vec::new();
		for must in &params.must_assign {
			let t = best_fit_portal_center(must.region, arc_degrees, closed);
			portals.push(AssignedPortal {
				t,
				portal: must.portal,
			});
		}

		let optional_n = optional_count(&noise, params.optional_portals);
		if optional_n > 0 {
			let candidates = can_assign_centers(
				arc_degrees,
				closed,
				&portals,
				&params.must_assign,
				&params.must_not_assign,
			);
			place_optional_portals(&noise, arc_degrees, &mut portals, &candidates, optional_n);
		}

		portals.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));

		let walls = tessellate_arc(
			params.center_xz,
			radius,
			storey_height,
			arc_degrees,
			closed,
			&portals,
		);
		Self {
			center_xz: params.center_xz,
			radius,
			storey_height,
			arc_degrees,
			portals,
			walls,
		}
	}
}

fn is_closed(arc_degrees: f32) -> bool {
	(arc_degrees - 360.0).abs() < 0.5
}

fn portal_half_t(arc_degrees: f32) -> f32 {
	OPEN_HALF_DEG / arc_degrees.max(SEG_DEG)
}

fn portal_width_t(arc_degrees: f32) -> f32 {
	PORTAL_WIDTH_DEG / arc_degrees.max(SEG_DEG)
}

fn seg_count(arc_degrees: f32) -> u32 {
	((arc_degrees / SEG_DEG).round() as u32).max(1)
}

fn optional_count(noise: &NoiseConfig, (min, max): (u32, u32)) -> u32 {
	let min = min as usize;
	let max = max as usize;
	if max < min {
		return min as u32;
	}
	// Inclusive max → sample in [min, max+1).
	noise.sample_range_usize_4d(min, max + 1, 0.17, 0.0, 0.0, 1.0) as u32
}

fn best_fit_portal_center(region: ArcRegion, arc_degrees: f32, closed: bool) -> f32 {
	snap_portal_center(region.midpoint(), arc_degrees, closed)
}

fn snap_portal_center(t: f32, arc_degrees: f32, closed: bool) -> f32 {
	let n = seg_count(arc_degrees) as f32;
	let mut snapped = (norm_t(t) * n).round().rem_euclid(n) / n;
	if !closed {
		let half = portal_half_t(arc_degrees);
		snapped = snapped.clamp(half, (1.0 - half).max(half));
	}
	snapped
}

fn portal_interval(center: f32, arc_degrees: f32) -> ArcRegion {
	let half = portal_half_t(arc_degrees);
	ArcRegion::span(center - half, center + half)
}

/// Kit-aligned centers whose portal footprint lies in can-assign space.
fn can_assign_centers(
	arc_degrees: f32,
	closed: bool,
	placed: &[AssignedPortal],
	must_assign: &[MustAssignPortal],
	must_not: &[ArcRegion],
) -> Vec<f32> {
	let mut blocked: Vec<ArcRegion> = must_assign.iter().map(|m| m.region).collect();
	blocked.extend(must_not.iter().copied());
	for p in placed {
		blocked.push(portal_interval(p.t, arc_degrees));
	}

	let n = seg_count(arc_degrees);
	let half = portal_half_t(arc_degrees);
	(0..n)
		.map(|i| i as f32 / n as f32)
		.filter(|&c| {
			if !closed && (c < half - 1e-5 || c > 1.0 - half + 1e-5) {
				return false;
			}
			// Portal must fit on an open arc (no wrap past the ends).
			if !closed {
				let w = portal_width_t(arc_degrees);
				if c - half < -1e-5 || c + half > 1.0 + 1e-5 || w > 1.0 + 1e-5 {
					return false;
				}
			}
			let foot = portal_interval(c, arc_degrees);
			!blocked.iter().any(|b| regions_overlap(foot, *b, closed))
		})
		.collect()
}

fn place_optional_portals(
	noise: &NoiseConfig,
	arc_degrees: f32,
	portals: &mut Vec<AssignedPortal>,
	candidates: &[f32],
	count: u32,
) {
	if count == 0 || candidates.is_empty() {
		return;
	}

	let mut scored: Vec<(f32, f32)> = candidates
		.iter()
		.copied()
		.map(|t| {
			let (s, c) = (t * std::f32::consts::TAU).sin_cos();
			let score = noise.sample_unit_4d(c, 0.0, s, 2.0);
			(score, t)
		})
		.collect();
	scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

	let mut blocked: Vec<ArcRegion> = portals
		.iter()
		.map(|p| portal_interval(p.t, arc_degrees))
		.collect();
	let mut placed = 0u32;
	for (_, t) in scored {
		if placed >= count {
			break;
		}
		let foot = portal_interval(t, arc_degrees);
		if blocked.iter().any(|b| regions_overlap(foot, *b, true)) {
			continue;
		}
		portals.push(AssignedPortal {
			t,
			portal: Portal::Window,
		});
		blocked.push(foot);
		placed += 1;
	}
}

fn tessellate_arc(
	center_xz: Vec3,
	radius: f32,
	storey_height: f32,
	arc_degrees: f32,
	closed: bool,
	portals: &[AssignedPortal],
) -> Vec<Placed<Wall>> {
	let ring_scale = Vec3::new(radius, storey_height, radius);
	let lintel = center_xz + Vec3::Y * (HEADER_Y_FRAC * storey_height);
	let mut walls = Vec::new();

	for portal in portals {
		let center_deg = portal.t * arc_degrees;
		let open_start = center_deg - OPEN_HALF_DEG;
		for i in 0..PORTAL_SEGS {
			let seg_start = if closed {
				norm_deg(open_start + i as f32 * SEG_DEG)
			} else {
				(open_start + i as f32 * SEG_DEG).clamp(0.0, arc_degrees - SEG_DEG)
			};
			let yaw = seg_start.to_radians();
			match portal.portal {
				Portal::Door => {
					walls.push(
						Placed::new(Wall::header_arc(SEG_DEG), lintel, yaw).with_scale(ring_scale),
					);
				}
				Portal::Window => {
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

	if portals.is_empty() {
		push_solid_sweep(&mut walls, center_xz, ring_scale, 0.0, arc_degrees);
		return walls;
	}

	if closed {
		for i in 0..portals.len() {
			let c0 = portals[i].t * arc_degrees;
			let c1 = portals[(i + 1) % portals.len()].t * arc_degrees;
			let solid_start = norm_deg(c0 + OPEN_HALF_DEG);
			let solid_end = norm_deg(c1 - OPEN_HALF_DEG);
			let sweep = if solid_end >= solid_start - 1e-3 {
				solid_end - solid_start
			} else {
				solid_end + arc_degrees - solid_start
			};
			push_solid_sweep(&mut walls, center_xz, ring_scale, solid_start, sweep);
		}
	} else {
		let first = portals[0].t * arc_degrees;
		let last = portals[portals.len() - 1].t * arc_degrees;
		push_solid_sweep(
			&mut walls,
			center_xz,
			ring_scale,
			0.0,
			(first - OPEN_HALF_DEG).max(0.0),
		);
		for i in 0..portals.len().saturating_sub(1) {
			let c0 = portals[i].t * arc_degrees;
			let c1 = portals[i + 1].t * arc_degrees;
			let solid_start = c0 + OPEN_HALF_DEG;
			let solid_end = c1 - OPEN_HALF_DEG;
			push_solid_sweep(
				&mut walls,
				center_xz,
				ring_scale,
				solid_start,
				(solid_end - solid_start).max(0.0),
			);
		}
		push_solid_sweep(
			&mut walls,
			center_xz,
			ring_scale,
			last + OPEN_HALF_DEG,
			(arc_degrees - (last + OPEN_HALF_DEG)).max(0.0),
		);
	}

	walls
}

fn push_solid_sweep(
	walls: &mut Vec<Placed<Wall>>,
	center_xz: Vec3,
	ring_scale: Vec3,
	start_deg: f32,
	sweep_deg: f32,
) {
	if sweep_deg > 1e-2 {
		walls.push(
			Placed::new(Wall::arc(sweep_deg), center_xz, start_deg.to_radians())
				.with_scale(ring_scale),
		);
	}
}

fn norm_t(t: f32) -> f32 {
	let mut t = t % 1.0;
	if t < 0.0 {
		t += 1.0;
	}
	t
}

fn norm_deg(deg: f32) -> f32 {
	let mut d = deg % 360.0;
	if d < 0.0 {
		d += 360.0;
	}
	d
}

fn circular_dist(a: f32, b: f32) -> f32 {
	let d = (norm_t(a) - norm_t(b)).abs();
	d.min(1.0 - d)
}

fn regions_overlap(a: ArcRegion, b: ArcRegion, allow_wrap: bool) -> bool {
	if a.is_point() && b.is_point() {
		return circular_dist(a.start, b.start) < 1e-5;
	}
	if a.is_point() {
		return b.contains_t(a.start);
	}
	if b.is_point() {
		return a.contains_t(b.start);
	}
	if !allow_wrap && (a.end < a.start || b.end < b.start) {
		// Treat wrap as split only when closed; otherwise clamp to linear.
	}
	interval_overlap_unwrap(a, b)
}

fn interval_overlap_unwrap(a: ArcRegion, b: ArcRegion) -> bool {
	fn unwrap(r: ArcRegion) -> Vec<(f32, f32)> {
		if r.is_point() {
			return vec![(r.start, r.start)];
		}
		if r.end >= r.start {
			vec![(r.start, r.end)]
		} else {
			vec![(r.start, 1.0), (0.0, r.end)]
		}
	}
	for (a0, a1) in unwrap(a) {
		for (b0, b1) in unwrap(b) {
			if a0 < b1 && b0 < a1 {
				return true;
			}
		}
	}
	false
}

#[cfg(test)]
mod tests {
	use super::*;

	fn cardinal_must_assign() -> Vec<MustAssignPortal> {
		vec![
			MustAssignPortal::at(0.0, Portal::Door),
			MustAssignPortal::at(0.25, Portal::Window),
			MustAssignPortal::at(0.5, Portal::Window),
			MustAssignPortal::at(0.75, Portal::Window),
		]
	}

	fn closed_arc(optional: (u32, u32), seed: i32) -> ArcWall {
		ArcWall::new(ArcWallParams {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			arc_degrees: 360.0,
			must_assign: cardinal_must_assign(),
			must_not_assign: vec![],
			portal_noise: NoiseParams {
				seed,
				..NoiseParams::default()
			},
			optional_portals: optional,
		})
	}

	#[test]
	fn must_assign_cardinals_without_optional() -> anyhow::Result<()> {
		let wall = closed_arc((0, 0), 1);
		assert_eq!(wall.portals.len(), 4);
		assert!(matches!(wall.portals[0].portal, Portal::Door));
		assert!((wall.portals[0].t - 0.0).abs() < 1e-5);
		assert!((wall.portals[1].t - 0.25).abs() < 1e-5);
		assert!((wall.arc_degrees - 360.0).abs() < 1e-3);
		let headers = wall
			.walls
			.iter()
			.filter(|w| matches!(w.geom, Wall::HeaderArc(_)))
			.count();
		assert_eq!(headers, 14);
		Ok(())
	}

	#[test]
	fn optional_portals_stay_in_can_assign() -> anyhow::Result<()> {
		let wall = closed_arc((0, 4), 42);
		assert!(wall.portals.len() >= 4);
		assert!(wall.portals.len() <= 8);
		for i in 0..wall.portals.len() {
			for j in (i + 1)..wall.portals.len() {
				let a = portal_interval(wall.portals[i].t, wall.arc_degrees);
				let b = portal_interval(wall.portals[j].t, wall.arc_degrees);
				assert!(
					!regions_overlap(a, b, true),
					"portals {} and {} overlap",
					wall.portals[i].t,
					wall.portals[j].t
				);
			}
		}
		Ok(())
	}

	#[test]
	fn must_not_blocks_optional() -> anyhow::Result<()> {
		let wall = ArcWall::new(ArcWallParams {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			arc_degrees: 360.0,
			must_assign: vec![MustAssignPortal::at(0.0, Portal::Door)],
			must_not_assign: vec![ArcRegion::span(0.1, 0.9)],
			portal_noise: NoiseParams {
				seed: 7,
				..NoiseParams::default()
			},
			optional_portals: (4, 4),
		});
		assert_eq!(wall.portals.len(), 1);
		Ok(())
	}

	#[test]
	fn open_half_arc_has_no_wrap_solid() -> anyhow::Result<()> {
		let wall = ArcWall::new(ArcWallParams {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			arc_degrees: 180.0,
			must_assign: vec![
				MustAssignPortal::at(0.25, Portal::Window),
				MustAssignPortal::at(0.75, Portal::Window),
			],
			must_not_assign: vec![],
			portal_noise: NoiseParams::default(),
			optional_portals: (0, 0),
		});
		assert!((wall.arc_degrees - 180.0).abs() < 1e-3);
		assert_eq!(wall.portals.len(), 2);
		let solids = wall
			.walls
			.iter()
			.filter(|w| matches!(w.geom, Wall::Arc(_)))
			.count();
		assert_eq!(solids, 3);
		Ok(())
	}
}
