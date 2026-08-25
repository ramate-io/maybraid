//! Two-opening stairwell: horizontal shaft faces → run-in + fitted stair flight.
//!
//! Each [`StairwellOpening`] is a **horizontal** [`MappedOpening`] (shaft
//! cross-section). The pair describes a vertical well: `lower` is the floor-space
//! anchor landing, `upper` is the open top landing. On each quad the **lower**
//! edge is the walk-on (host-floor-connected); the **upper** edge is the far
//! side of the hole. `orientation` is XZ walk-off from that walk-on into the
//! well.
//!
//! The well owns a short floor run-in at the lower walk-on and, by default, an
//! upper landing: a thin slab flush with the last tread’s leading edge, then
//! along the nearby opening rim. Thickness is [`LANDING_THICKNESS_M`] unless
//! set with [`ConnectingStairwell::with_landing_thickness`]. Turn the landing
//! off when a follow-on stairwell will own that floor. The shaft is filled
//! with composed [`StairNode`]s. It does not author walls or emit shaft
//! opening labels. A [`FlightPolyline`] along face centers absorbs plan
//! offset; v1 always fits a [`SpiralFlight`] inside the lower opening.

use std::ops::Deref;

use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::connecting::geom::{normalize_xz, EPS};
use crate::openings::MappedOpening;
use crate::paneling::panel_complex::{PanelComplexJointPolicy, PanelPoint};
use crate::paneling::quad_panel::QuadPanel;
use crate::paneling::tube::{Tube, TubeCrossSectionNode, TubeFaces};
use crate::stair_flights::{FlightPolyline, FlightStation, SpiralFlight, SpiralFlightFit};

/// Aesthetic run-in depth from the lower walk-on into the shaft (meters).
pub const RUN_IN_M: f32 = 0.75;

/// Plan separation below which the polyline stays a single vertical segment.
const PLAN_KINK_EPS: f32 = 0.15;

/// Thin tube height so a landing presents as a floor, not a wall.
const RUN_IN_THICK: f32 = 0.05;

/// Default upper-landing kit thickness (meters).
pub const LANDING_THICKNESS_M: f32 = 0.05;

/// Minimum headroom above a tread before that plan is treated as landable.
const LANDING_CLEARANCE_M: f32 = 1.8;

/// Horizontal shaft-face opening, typed for [`ConnectingStairwell`].
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

/// Thin landing slab on the upper opening rim, flush with the last tread.
#[derive(Debug, Clone, PartialEq)]
pub struct StairwellLanding {
	panel: QuadPanel,
	thickness: f32,
	outer_start: Vec3,
	outer_end: Vec3,
	inner_start: Vec3,
	inner_end: Vec3,
}

impl StairwellLanding {
	pub fn thickness(&self) -> f32 {
		self.thickness
	}

	pub fn outer_start(&self) -> Vec3 {
		self.outer_start
	}

	pub fn outer_end(&self) -> Vec3 {
		self.outer_end
	}

	pub fn inner_start(&self) -> Vec3 {
		self.inner_start
	}

	pub fn inner_end(&self) -> Vec3 {
		self.inner_end
	}

	pub fn panel(&self) -> &QuadPanel {
		&self.panel
	}
}

impl BuildingComponents for StairwellLanding {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.panel.as_complex().panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.panel.as_complex().joint_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::new()
	}

	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::new()
	}
}

/// Two horizontal shaft faces → run-in / optional upper landing + spiral flight.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectingStairwell {
	style: PanelStyle,
	lower: StairwellOpening,
	upper: StairwellOpening,
	polyline: FlightPolyline,
	run_in: Tube,
	landing_enabled: bool,
	landing_thickness: f32,
	upper_landing: Option<StairwellLanding>,
	flight: SpiralFlight,
}

impl ConnectingStairwell {
	/// `lower` is the floor-space anchor even when both faces share a Y.
	pub fn new(
		style: PanelStyle,
		lower: impl Into<StairwellOpening>,
		upper: impl Into<StairwellOpening>,
	) -> Self {
		let lower = lower.into();
		let upper = upper.into();
		let polyline = build_polyline(lower, upper);
		let run_in = build_run_in(style, lower);
		let (lower_hw, lower_hd) = lower.plan_half_extents();
		let (upper_hw, upper_hd) = upper.plan_half_extents();
		let flight = SpiralFlight::fit(
			polyline.clone(),
			SpiralFlightFit {
				lower_center: lower.face_center(),
				upper_center: upper.face_center(),
				lower_walk_on: lower.walk_on_mid(),
				upper_walk_on: upper.walk_on_mid(),
				lower_out: lower.orientation,
				lower_half_width: lower_hw,
				lower_half_depth: lower_hd,
				upper_half_width: upper_hw,
				upper_half_depth: upper_hd,
			},
		);
		let landing_thickness = LANDING_THICKNESS_M;
		let upper_landing = build_upper_landing(style, upper, &flight, landing_thickness);
		Self {
			style,
			lower,
			upper,
			polyline,
			run_in,
			landing_enabled: true,
			landing_thickness,
			upper_landing,
			flight,
		}
	}

	pub fn rough_stone(
		lower: impl Into<StairwellOpening>,
		upper: impl Into<StairwellOpening>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, lower, upper)
	}

	/// Thin rim on the upper opening boundary, past the last tread.
	///
	/// Default is `true`. Set `false` when a follow-on stairwell will own that
	/// landing (its lower run-in).
	pub fn with_upper_landing(mut self, enabled: bool) -> Self {
		self.landing_enabled = enabled;
		self.rebuild_upper_landing();
		self
	}

	/// Kit thickness of the upper landing slab (meters). Default [`LANDING_THICKNESS_M`].
	pub fn with_landing_thickness(mut self, thickness: f32) -> Self {
		self.landing_thickness = thickness.max(1e-4);
		self.rebuild_upper_landing();
		self
	}

	pub fn landing_thickness(&self) -> f32 {
		self.landing_thickness
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.run_in = std::mem::replace(&mut self.run_in, Tube::new(self.style))
			.with_joint_policy(joint_policy);
		if let Some(landing) = self.upper_landing.take() {
			self.upper_landing = Some(StairwellLanding {
				panel: landing.panel.with_joint_policy(joint_policy),
				thickness: landing.thickness,
				outer_start: landing.outer_start,
				outer_end: landing.outer_end,
				inner_start: landing.inner_start,
				inner_end: landing.inner_end,
			});
		}
		self
	}

	fn rebuild_upper_landing(&mut self) {
		self.upper_landing = if self.landing_enabled {
			build_upper_landing(self.style, self.upper, &self.flight, self.landing_thickness)
		} else {
			None
		};
	}

	pub fn lower(&self) -> StairwellOpening {
		self.lower
	}

	pub fn upper(&self) -> StairwellOpening {
		self.upper
	}

	pub fn polyline(&self) -> &FlightPolyline {
		&self.polyline
	}

	pub fn run_in(&self) -> &Tube {
		&self.run_in
	}

	pub fn upper_landing(&self) -> Option<&StairwellLanding> {
		self.upper_landing.as_ref()
	}

	pub fn flight(&self) -> &SpiralFlight {
		&self.flight
	}
}

impl BuildingComponents for ConnectingStairwell {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.run_in.panel_nodes_for_level(level);
		if let Some(landing) = &self.upper_landing {
			out.extend(landing.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.run_in.joint_nodes_for_level(level);
		if let Some(landing) = &self.upper_landing {
			out.extend(landing.joint_nodes_for_level(level));
		}
		out
	}

	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::new()
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		self.flight.stair_nodes_for_level(level)
	}
}

fn build_polyline(lower: StairwellOpening, upper: StairwellOpening) -> FlightPolyline {
	let a = lower.face_center();
	let b = upper.face_center();
	let p_a = Vec2::new(a.x, a.z);
	let p_b = Vec2::new(b.x, b.z);
	let rise = (b.y - a.y).abs().max(EPS);

	let mut stations = vec![FlightStation { center: a, height: rise }];
	if (p_a - p_b).length() > PLAN_KINK_EPS {
		// Horizontal faces do not cast wall rays; a plan midpoint covers offset shafts.
		let m_xz = (p_a + p_b) * 0.5;
		let y = 0.5 * (a.y + b.y);
		stations.push(FlightStation { center: Vec3::new(m_xz.x, y, m_xz.y), height: rise });
	}
	stations.push(FlightStation { center: b, height: rise });
	FlightPolyline { stations }
}

fn build_run_in(style: PanelStyle, lower: StairwellOpening) -> Tube {
	let Some(out) = normalize_xz(lower.orientation) else {
		return Tube::new(style);
	};
	let walk = lower.walk_on_mid();
	let half_w = (0.5 * lower.walk_on_width()).max(EPS);
	let out3 = Vec3::new(out.x, 0.0, out.y) * RUN_IN_M;
	let node0 = TubeCrossSectionNode::new(walk, half_w, half_w, RUN_IN_THICK, half_w, half_w);
	let node1 =
		TubeCrossSectionNode::new(walk + out3, half_w, half_w, RUN_IN_THICK, half_w, half_w);
	Tube::from_nodes(style, [node0, node1]).with_faces(TubeFaces {
		floor: true,
		ceiling: false,
		left: false,
		right: false,
	})
}

/// Parallelogram flush with the last tread’s leading edge, extruded along the
/// nearby opening rim. The last tread is not a clearance blocker — you step
/// off it onto the landing.
fn build_upper_landing(
	style: PanelStyle,
	opening: StairwellOpening,
	flight: &SpiralFlight,
	thickness: f32,
) -> Option<StairwellLanding> {
	use richmond_building_components::stairs::Stair;

	let Stair::Spiral(g) = &flight.stairs().geometry else {
		return None;
	};
	let corners = opening_boundary_ccw(opening);
	let y = opening.walk_on_mid().y;
	let block_r = 0.5 * g.tread_depth + 0.02;
	let travel = flight.last_tread_travel_xz();
	let (lead_outer, lead_inner) = flight.last_tread_leading_xz();
	let (edge, a0_xz) = nearest_boundary(lead_outer, &corners)?;
	let a = corners[edge];
	let b = corners[(edge + 1) % 4];
	let edge_dir = (b - a).normalize_or_zero();
	if edge_dir.length_squared() < EPS * EPS {
		return None;
	}
	let along = if edge_dir.dot(travel) >= 0.0 { edge_dir } else { -edge_dir };
	let end_pt = if along.dot(b - a) >= 0.0 { b } else { a };
	let lead = lead_inner - lead_outer;
	let b0_xz = a0_xz + lead;
	if (end_pt - a0_xz).dot(along) < 0.12 {
		return None;
	}
	let step = 0.06;
	let mut end = a0_xz;
	let mut walked = 0.0;
	while walked < RUN_IN_M {
		let next = end + along * step;
		if (next - a0_xz).dot(along) > (end_pt - a0_xz).dot(along) {
			end = end_pt;
			break;
		}
		if !has_tread_clearance(flight, next + lead * 0.5, y, block_r) {
			break;
		}
		end = next;
		walked += step;
	}
	if (end - a0_xz).length() < 0.12 {
		return None;
	}
	let b1_xz = b0_xz + (end - a0_xz);
	let a0 = Vec3::new(a0_xz.x, y, a0_xz.y);
	let a1 = Vec3::new(end.x, y, end.y);
	let b0 = Vec3::new(b0_xz.x, y, b0_xz.y);
	let b1 = Vec3::new(b1_xz.x, y, b1_xz.y);
	let thick = |p: Vec3| PanelPoint::new(p, thickness);
	Some(StairwellLanding {
		panel: QuadPanel::new(style, thick(a0), thick(a1), thick(b0), thick(b1)),
		thickness,
		outer_start: a0,
		outer_end: a1,
		inner_start: b0,
		inner_end: b1,
	})
}

/// CCW hole boundary from above: walk-on, then the \(+\)right side, far, \(-\)right.
fn opening_boundary_ccw(opening: StairwellOpening) -> [Vec2; 4] {
	let (bl, br, tl, tr) = opening.endpoint_corners();
	[
		Vec2::new(bl.x, bl.z),
		Vec2::new(br.x, br.z),
		Vec2::new(tr.x, tr.z),
		Vec2::new(tl.x, tl.z),
	]
}

fn nearest_boundary(p: Vec2, corners: &[Vec2; 4]) -> Option<(usize, Vec2)> {
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

fn has_tread_clearance(flight: &SpiralFlight, p: Vec2, landing_y: f32, radius: f32) -> bool {
	let stations = flight.tread_stations();
	!stations.iter().rev().skip(1).any(|&(txz, top)| {
		landing_y - top < LANDING_CLEARANCE_M && (p - txz).length() < radius
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::stairs::Stair;

	fn dist_to_opening_boundary(opening: StairwellOpening, p: Vec2) -> f32 {
		let corners = opening_boundary_ccw(opening);
		let mut best = f32::MAX;
		for i in 0..4 {
			let a = corners[i];
			let b = corners[(i + 1) % 4];
			let v = b - a;
			let len2 = v.length_squared();
			if len2 < EPS * EPS {
				continue;
			}
			let u = ((p - a).dot(v) / len2).clamp(0.0, 1.0);
			best = best.min((p - (a + v * u)).length());
		}
		best
	}

	/// Horizontal shaft face: `center` in the hole, walk-on on the −orientation side.
	fn shaft_opening(
		center: Vec3,
		half_w: f32,
		half_d: f32,
		orient: Vec2,
	) -> anyhow::Result<MappedOpening> {
		let d = normalize_xz(orient)
			.ok_or_else(|| anyhow::anyhow!("orientation too short: {orient:?}"))?;
		let right = Vec3::new(-d.y, 0.0, d.x);
		let out = Vec3::new(d.x, 0.0, d.y);
		let walk = center - out * half_d;
		let far = center + out * half_d;
		let bl = walk - right * half_w;
		let br = walk + right * half_w;
		let tl = far - right * half_w;
		let tr = far + right * half_w;
		Ok(MappedOpening::from_corners(bl, br, tl, tr, orient))
	}

	#[test]
	fn stacked_shafts_use_two_polyline_stations() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		assert_eq!(well.polyline().stations.len(), 2);
		assert!((well.polyline().stations[0].center.y).abs() < 1e-3);
		assert!((well.polyline().stations[1].center.y - 3.0).abs() < 1e-3);
		assert!(well.polyline().rise() > 2.9);
		Ok(())
	}

	#[test]
	fn plan_offset_inserts_midpoint_station() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, -3.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(3.0, 3.0, 0.0), 1.2, 1.2, -Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		assert_eq!(well.polyline().stations.len(), 3);
		let mid = well.polyline().stations[1].center;
		assert!((mid.x - 1.5).abs() < 1e-3 && (mid.z + 1.5).abs() < 1e-3, "mid={mid:?}");
		assert!((mid.y - 1.5).abs() < 1e-3, "mid.y={}", mid.y);
		Ok(())
	}

	#[test]
	fn run_in_follows_walk_off_into_shaft() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::X)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		assert!(well.run_in().faces().floor);
		assert!(!well.run_in().faces().ceiling);
		assert!(!well.run_in().floor().pieces().is_empty());
		let inward =
			well.run_in().nodes()[1].bottom_middle - well.run_in().nodes()[0].bottom_middle;
		assert!(inward.x > 0.5, "run-in should follow +X into the shaft, got {inward:?}");
		Ok(())
	}

	#[test]
	fn upper_landing_follows_last_tread_travel() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, -Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let landing = well.upper_landing().expect("upper landing on by default");
		assert!((landing.thickness() - LANDING_THICKNESS_M).abs() < 1e-3);
		let a = landing.outer_start();
		let b = landing.outer_end();
		assert!((a.y - 3.0).abs() < 1e-3);
		for p in [a, b] {
			assert!(
				p.x.abs() <= 1.2 + 1e-3 && p.z.abs() <= 1.2 + 1e-3,
				"landing must stay in the opening, {p:?}"
			);
			assert!(
				dist_to_opening_boundary(well.upper(), Vec2::new(p.x, p.z)) < 0.04,
				"landing should sit on the opening boundary, {p:?}"
			);
		}
		let travel = well.flight().last_tread_travel_xz();
		let last = well.flight().last_tread_xz();
		let (lead_outer, lead_inner) = well.flight().last_tread_leading_xz();
		let start = Vec2::new(a.x, a.z);
		assert!(
			(start - last).length() < 1.0,
			"landing should stay next to the last tread, not across the hole, last={last:?} start={start:?}"
		);
		assert!(
			(start - lead_outer).length() < 0.12,
			"landing should meet the last-tread leading outer, lead={lead_outer:?} start={start:?}"
		);
		let start_edge = Vec2::new(landing.inner_start().x - a.x, landing.inner_start().z - a.z);
		let leading = lead_inner - lead_outer;
		assert!(
			start_edge.normalize_or_zero().dot(leading.normalize_or_zero()) > 0.9,
			"landing start should be flush with the last-tread leading edge"
		);
		let along = Vec2::new(b.x - a.x, b.z - a.z);
		assert!(
			along.dot(travel) > 0.0,
			"landing should continue in the travel half-plane, travel={travel:?} along={along:?}"
		);
		Ok(())
	}

	#[test]
	fn landing_thickness_is_overridable() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper).with_landing_thickness(0.12);
		assert!((well.landing_thickness() - 0.12).abs() < 1e-4);
		assert!((well.upper_landing().expect("landing").thickness() - 0.12).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn omitting_upper_landing_leaves_only_run_in() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let with = ConnectingStairwell::rough_stone(lower, upper);
		let without = ConnectingStairwell::rough_stone(lower, upper).with_upper_landing(false);
		assert!(with.upper_landing().is_some());
		assert!(without.upper_landing().is_none());
		let with_n = with.panel_nodes_for_level(LodSceneLevel::High).flatten().len();
		let without_n = without.panel_nodes_for_level(LodSceneLevel::High).flatten().len();
		assert!(with_n > without_n, "upper landing should add floor panels ({with_n} vs {without_n})");
		Ok(())
	}

	#[test]
	fn fills_spiral_inside_shaft() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let stairs = well.stair_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(stairs.len(), 1);
		assert!(
			matches!(&stairs[0].geometry, Stair::Spiral(g) if g.height > 2.9 && (g.radius + 0.5 * g.tread_width - 1.2).abs() < 1e-3)
		);
		let center = stairs[0].placement.translation;
		assert!(
			center.x.abs() < 0.2 && center.z.abs() < 0.2,
			"spiral should sit in the shaft, got {center:?}"
		);
		assert!(well.run_in().faces().left == false && well.run_in().faces().right == false);
		Ok(())
	}

	#[test]
	fn same_y_keeps_explicit_lower_as_anchor() -> anyhow::Result<()> {
		let a = shaft_opening(Vec3::new(-2.0, 1.0, 0.0), 1.0, 1.0, Vec2::X)?;
		let b = shaft_opening(Vec3::new(2.0, 1.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(a, b);
		assert!((well.lower().face_center().x + 2.0).abs() < 1e-3);
		assert!((well.upper().face_center().x - 2.0).abs() < 1e-3);
		assert!(well.polyline().rise().abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn walk_on_is_lower_edge_of_horizontal_face() -> anyhow::Result<()> {
		let opening = shaft_opening(Vec3::new(0.0, 1.5, 0.0), 1.0, 1.2, Vec2::X)?;
		let end = StairwellOpening::new(opening);
		let walk = end.walk_on_mid();
		assert!((walk.y - 1.5).abs() < 1e-3);
		assert!(walk.x < end.face_center().x - 0.5, "walk-on should sit on −X of an +X opening");
		let (hw, hd) = end.plan_half_extents();
		assert!((hw - 1.0).abs() < 1e-3);
		assert!((hd - 1.2).abs() < 1e-3);
		Ok(())
	}
}
