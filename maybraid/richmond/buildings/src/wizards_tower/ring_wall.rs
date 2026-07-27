//! Parameterized circular ring wall with portal (door/window) openings.
//!
//! Construction:
//! 1. **Must-assign** — best-fit a portal into each required region.
//! 2. **Can-assign** — the unit ring minus the union of must-assign and must-not
//!    regions (plus footprints of portals already placed).
//! 3. **Optional** — sample portal noise for how many optional portals to attempt
//!    in \([min, max]\) and where to place them in can-assign space.

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::partitions::Wall;
use richmond_building_components::Placed;

/// Kit segment size (degrees) and portal width (two segments → 30°).
const SEG_DEG: f32 = 15.0;
const SEGS: u32 = 24;
const PORTAL_SEGS: u32 = 2;
/// Portal width in normalized arc parameter \(t \in [0, 1)\).
const PORTAL_WIDTH_T: f32 = PORTAL_SEGS as f32 / SEGS as f32;
const PORTAL_HALF_T: f32 = PORTAL_WIDTH_T * 0.5;
/// Lintel / top-header baseline as a fraction of storey height.
const HEADER_Y_FRAC: f32 = 0.7;

/// Opening cut into a ring wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Portal {
	Door,
	Window,
}

/// Inclusive–exclusive interval on the unit circle \(t \in [0, 1)\).
///
/// When `start == end` the region is a **point** locus (preferred / forbidden \(t\)).
/// When `start > end` the interval wraps across \(0\).
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

	/// Half-open span `[start, end)` on the unit circle (wraps if `start > end`).
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

/// Parameters for [`RingWall::new`].
#[derive(Debug, Clone)]
pub struct RingWallParams {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	/// Regions that **must** receive a portal (best-fit).
	pub must_assign: Vec<MustAssignPortal>,
	/// Regions that **must not** receive a portal.
	pub must_not_assign: Vec<ArcRegion>,
	/// Noise used for optional portal count and placement.
	pub portal_noise: NoiseParams,
	/// Inclusive \((min, max)\) optional portals to attempt in can-assign space.
	pub optional_portals: (u32, u32),
}

/// Portal assigned on the ring (center \(t\), kind).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssignedPortal {
	pub t: f32,
	pub portal: Portal,
}

/// Circular outer wall with door/window openings.
#[derive(Debug, Clone, PartialEq)]
pub struct RingWall {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	pub portals: Vec<AssignedPortal>,
	pub walls: Vec<Placed<Wall>>,
}

impl RingWall {
	/// Assign must portals, then noise-sample optional portals in can-assign regions.
	pub fn new(params: RingWallParams) -> Self {
		let radius = params.radius.max(1e-4);
		let storey_height = params.storey_height.max(1e-4);
		let noise = NoiseConfig::new(params.portal_noise);

		let mut portals = Vec::new();
		for must in &params.must_assign {
			let t = best_fit_portal_center(must.region);
			portals.push(AssignedPortal {
				t,
				portal: must.portal,
			});
		}

		let optional_n = optional_count(&noise, params.optional_portals);
		if optional_n > 0 {
			let candidates = can_assign_centers(&portals, &params.must_assign, &params.must_not_assign);
			place_optional_portals(&noise, &mut portals, &candidates, optional_n);
		}

		portals.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));

		let walls = tessellate_ring(params.center_xz, radius, storey_height, &portals);
		Self {
			center_xz: params.center_xz,
			radius,
			storey_height,
			portals,
			walls,
		}
	}
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

fn best_fit_portal_center(region: ArcRegion) -> f32 {
	snap_portal_center(region.midpoint())
}

fn snap_portal_center(t: f32) -> f32 {
	let n = SEGS as f32;
	(norm_t(t) * n).round().rem_euclid(n) / n
}

fn portal_interval(center: f32) -> ArcRegion {
	ArcRegion::span(center - PORTAL_HALF_T, center + PORTAL_HALF_T)
}

/// Kit-aligned centers whose portal footprint lies in can-assign space.
fn can_assign_centers(
	placed: &[AssignedPortal],
	must_assign: &[MustAssignPortal],
	must_not: &[ArcRegion],
) -> Vec<f32> {
	let mut blocked: Vec<ArcRegion> = must_assign.iter().map(|m| m.region).collect();
	blocked.extend(must_not.iter().copied());
	for p in placed {
		blocked.push(portal_interval(p.t));
	}

	(0..SEGS)
		.map(|i| i as f32 / SEGS as f32)
		.filter(|&c| {
			let foot = portal_interval(c);
			!blocked.iter().any(|b| regions_overlap(foot, *b))
		})
		.collect()
}

fn place_optional_portals(
	noise: &NoiseConfig,
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

	let mut blocked: Vec<ArcRegion> = portals.iter().map(|p| portal_interval(p.t)).collect();
	let mut placed = 0u32;
	for (_, t) in scored {
		if placed >= count {
			break;
		}
		let foot = portal_interval(t);
		if blocked.iter().any(|b| regions_overlap(foot, *b)) {
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

fn tessellate_ring(
	center_xz: Vec3,
	radius: f32,
	storey_height: f32,
	portals: &[AssignedPortal],
) -> Vec<Placed<Wall>> {
	let ring_scale = Vec3::new(radius, storey_height, radius);
	let lintel = center_xz + Vec3::Y * (HEADER_Y_FRAC * storey_height);
	let mut walls = Vec::new();

	for portal in portals {
		let center_deg = portal.t * 360.0;
		let open_start = center_deg - OPEN_HALF_DEG;
		for i in 0..PORTAL_SEGS {
			let seg_start = norm_deg(open_start + i as f32 * SEG_DEG);
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
		walls.push(Placed::new(Wall::arc(180.0), center_xz, 0.0).with_scale(ring_scale));
		walls.push(
			Placed::new(Wall::arc(180.0), center_xz, std::f32::consts::PI).with_scale(ring_scale),
		);
		return walls;
	}

	for i in 0..portals.len() {
		let c0 = portals[i].t * 360.0;
		let c1 = portals[(i + 1) % portals.len()].t * 360.0;
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

/// Half-width of each portal in degrees (two 15° segments → 30° total).
const OPEN_HALF_DEG: f32 = SEG_DEG * (PORTAL_SEGS as f32) * 0.5;

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

fn regions_overlap(a: ArcRegion, b: ArcRegion) -> bool {
	if a.is_point() && b.is_point() {
		return circular_dist(a.start, b.start) < 1e-5;
	}
	if a.is_point() {
		return b.contains_t(a.start);
	}
	if b.is_point() {
		return a.contains_t(b.start);
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

/// Cardinal door + windows used by the Wizard's Tower storeys.
pub fn wizard_tower_must_assign() -> Vec<MustAssignPortal> {
	vec![
		MustAssignPortal::at(0.0, Portal::Door),
		MustAssignPortal::at(0.25, Portal::Window),
		MustAssignPortal::at(0.5, Portal::Window),
		MustAssignPortal::at(0.75, Portal::Window),
	]
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tower_ring(optional: (u32, u32), seed: i32) -> RingWall {
		RingWall::new(RingWallParams {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			must_assign: wizard_tower_must_assign(),
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
		let ring = tower_ring((0, 0), 1);
		assert_eq!(ring.portals.len(), 4);
		assert!(matches!(ring.portals[0].portal, Portal::Door));
		assert!((ring.portals[0].t - 0.0).abs() < 1e-5);
		assert!((ring.portals[1].t - 0.25).abs() < 1e-5);
		let headers = ring
			.walls
			.iter()
			.filter(|w| matches!(w.geom, Wall::HeaderArc(_)))
			.count();
		assert_eq!(headers, 14);
		Ok(())
	}

	#[test]
	fn optional_portals_stay_in_can_assign() -> anyhow::Result<()> {
		let ring = tower_ring((0, 4), 42);
		assert!(ring.portals.len() >= 4);
		assert!(ring.portals.len() <= 8);
		// No two portal footprints overlap.
		for i in 0..ring.portals.len() {
			for j in (i + 1)..ring.portals.len() {
				let a = portal_interval(ring.portals[i].t);
				let b = portal_interval(ring.portals[j].t);
				assert!(
					!regions_overlap(a, b),
					"portals {} and {} overlap",
					ring.portals[i].t,
					ring.portals[j].t
				);
			}
		}
		Ok(())
	}

	#[test]
	fn must_not_blocks_optional() -> anyhow::Result<()> {
		let ring = RingWall::new(RingWallParams {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			must_assign: vec![MustAssignPortal::at(0.0, Portal::Door)],
			must_not_assign: vec![
				ArcRegion::span(0.1, 0.9), // almost whole ring blocked
			],
			portal_noise: NoiseParams {
				seed: 7,
				..NoiseParams::default()
			},
			optional_portals: (4, 4),
		});
		assert_eq!(ring.portals.len(), 1);
		Ok(())
	}
}
