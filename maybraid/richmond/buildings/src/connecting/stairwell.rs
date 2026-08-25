//! Two-opening stairwell: horizontal shaft faces → run-in + fitted stair flight.
//!
//! Each [`StairwellOpening`] is a **horizontal** [`MappedOpening`] (shaft
//! cross-section). The pair describes a vertical well: `lower` is the floor-space
//! anchor landing, `upper` is the open top landing. On each quad the **lower**
//! edge is the walk-on (host-floor-connected); the **upper** edge is the far
//! side of the hole. `orientation` is XZ walk-off from that walk-on into the
//! well.
//!
//! The well owns a short floor run-in at the lower walk-on and fills the shaft
//! with composed [`StairNode`]s. It does not author walls or emit shaft opening
//! labels. A [`FlightPolyline`] along face centers absorbs plan offset; v1
//! always fits a [`SpiralFlight`] inside the lower opening.

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
use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::tube::{Tube, TubeCrossSectionNode, TubeFaces};
use crate::stair_flights::{FlightPolyline, FlightStation, SpiralFlight, SpiralFlightFit};

/// Aesthetic run-in depth from the lower walk-on into the shaft (meters).
pub const RUN_IN_M: f32 = 0.75;

/// Plan separation below which the polyline stays a single vertical segment.
const PLAN_KINK_EPS: f32 = 0.15;

/// Thin tube height so the run-in presents as a floor, not a wall.
const RUN_IN_THICK: f32 = 0.05;

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

/// Two horizontal shaft faces → run-in floor + spiral flight (no well walls).
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectingStairwell {
	style: PanelStyle,
	lower: StairwellOpening,
	upper: StairwellOpening,
	polyline: FlightPolyline,
	run_in: Tube,
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
		Self { style, lower, upper, polyline, run_in, flight }
	}

	pub fn rough_stone(
		lower: impl Into<StairwellOpening>,
		upper: impl Into<StairwellOpening>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, lower, upper)
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.run_in = std::mem::replace(&mut self.run_in, Tube::new(self.style))
			.with_joint_policy(joint_policy);
		self
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

	pub fn flight(&self) -> &SpiralFlight {
		&self.flight
	}
}

impl BuildingComponents for ConnectingStairwell {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.run_in.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.run_in.joint_nodes_for_level(level)
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

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::stairs::Stair;

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
	fn fills_spiral_inside_shaft() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let stairs = well.stair_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(stairs.len(), 1);
		assert!(
			matches!(&stairs[0].geometry, Stair::Spiral(g) if g.height > 2.9 && (g.radius - 1.2).abs() < 1e-3)
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
