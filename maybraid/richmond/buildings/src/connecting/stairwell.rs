//! Two-opening stairwell: owned run-in floor and a fitted stair flight.
//!
//! [`StairwellOpening`] wraps the same [`MappedOpening`] contact as a hall end.
//! The **lower** edge of each quad is the floor-space-connected walk-on (anchor
//! at [`ConnectingStairwell::new`]'s `lower` argument). The **upper** edge is
//! not floor-connected (lintel). Circulation is sided: start at the lower
//! opening's `lower_left`, arrive at the upper opening's `lower_right`.
//!
//! The well owns a short floor run-in at the anchor and fills the volume up to
//! the top landing with composed [`StairNode`]s. It does not author walls or
//! emit shaft opening labels. A [`FlightPolyline`] absorbs plan offset between
//! the ends; v1 always fits a [`SpiralFlight`].

use std::ops::Deref;

use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::connecting::geom::{normalize_xz, opening_to_tube_node, plan_kink, EPS};
use crate::openings::MappedOpening;
use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::tube::{Tube, TubeCrossSectionNode, TubeFaces};
use crate::stair_flights::{FlightPolyline, FlightStation, SpiralFlight, SpiralFlightFit};

/// Aesthetic run-in depth along the lower opening's outward orientation (meters).
pub const RUN_IN_M: f32 = 0.75;

/// Plan separation below which the polyline stays a single vertical-ish segment.
const PLAN_KINK_EPS: f32 = 0.15;

/// Vertical connector opening: same contact as [`MappedOpening`], typed for
/// [`ConnectingStairwell`].
///
/// Lower quad edge = walk-on (floor-space-connected). `orientation` is XZ
/// outward into the well.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairwellOpening(MappedOpening);

impl StairwellOpening {
	pub fn new(mapped: MappedOpening) -> Self {
		Self(mapped)
	}

	pub fn mapped(self) -> MappedOpening {
		self.0
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

	/// Clear headroom from walk-on mid to lintel mid (meters).
	pub fn headroom(self) -> f32 {
		let (bl, br, tl, tr) = self.endpoint_corners();
		let bottom = (bl + br) * 0.5;
		let top = (tl + tr) * 0.5;
		(top.y - bottom.y).abs().max(EPS)
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

/// Two wall openings → run-in floor + spiral flight (no well walls).
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
	/// `lower` is the floor-space anchor even when both walk-ons share a Y.
	pub fn new(
		style: PanelStyle,
		lower: impl Into<StairwellOpening>,
		upper: impl Into<StairwellOpening>,
	) -> Self {
		let lower = lower.into();
		let upper = upper.into();
		let polyline = build_polyline(lower, upper);
		let run_in = build_run_in(style, lower);
		let flight = SpiralFlight::fit(
			polyline.clone(),
			SpiralFlightFit {
				lower_walk_on: lower.walk_on_mid(),
				upper_walk_on: upper.walk_on_mid(),
				lower_out: lower.orientation,
				lower_width: lower.walk_on_width(),
				upper_width: upper.walk_on_width(),
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
	let a = lower.walk_on_mid();
	let b = upper.walk_on_mid();
	let p_a = Vec2::new(a.x, a.z);
	let p_b = Vec2::new(b.x, b.z);
	let d_a = normalize_xz(lower.orientation);
	let d_b = normalize_xz(upper.orientation);
	let h_a = lower.headroom();
	let h_b = upper.headroom();

	let mut stations = vec![FlightStation { center: a, height: h_a }];
	if (p_a - p_b).length() > PLAN_KINK_EPS {
		let m_xz = plan_kink(p_a, d_a, p_b, d_b);
		let l_a = (m_xz - p_a).length().max(EPS);
		let l_b = (m_xz - p_b).length().max(EPS);
		let inv = 1.0 / (l_a + l_b);
		let w_a = l_b * inv;
		let w_b = l_a * inv;
		let y = w_a * a.y + w_b * b.y;
		let height = w_a * h_a + w_b * h_b;
		stations.push(FlightStation { center: Vec3::new(m_xz.x, y, m_xz.y), height });
	}
	stations.push(FlightStation { center: b, height: h_b });
	FlightPolyline { stations }
}

fn build_run_in(style: PanelStyle, lower: StairwellOpening) -> Tube {
	let Some(node0) = opening_to_tube_node(lower.mapped()) else {
		return Tube::new(style);
	};
	let Some(out) = normalize_xz(lower.orientation) else {
		return Tube::new(style);
	};
	let out3 = Vec3::new(out.x, 0.0, out.y) * RUN_IN_M;
	let mut node1 = TubeCrossSectionNode::new(
		node0.bottom_middle + out3,
		node0.bottom_left_width,
		node0.bottom_right_width,
		node0.height,
		node0.top_left_width,
		node0.top_right_width,
	);
	if let Some(top) = node0.top_middle {
		node1 = node1.with_top_middle(top + out3);
	}
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

	fn opening_facing(
		center: Vec3,
		half_w: f32,
		half_h: f32,
		orient: Vec2,
	) -> anyhow::Result<MappedOpening> {
		let d = normalize_xz(orient)
			.ok_or_else(|| anyhow::anyhow!("orientation too short: {orient:?}"))?;
		let right = Vec3::new(-d.y, 0.0, d.x);
		let up = Vec3::Y;
		let bl = center - right * half_w;
		let br = center + right * half_w;
		let tl = bl + up * (half_h * 2.0);
		let tr = br + up * (half_h * 2.0);
		Ok(MappedOpening::from_corners(bl, br, tl, tr, orient))
	}

	#[test]
	fn stacked_openings_use_two_polyline_stations() -> anyhow::Result<()> {
		let lower = opening_facing(Vec3::new(0.0, 0.0, 0.0), 1.0, 1.1, Vec2::X)?;
		let upper = opening_facing(Vec3::new(0.0, 3.0, 0.0), 1.0, 1.1, Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		assert_eq!(well.polyline().stations.len(), 2);
		assert!((well.polyline().stations[0].center.y).abs() < 1e-3);
		assert!((well.polyline().stations[1].center.y - 3.0).abs() < 1e-3);
		assert!(well.polyline().rise() > 2.9);
		Ok(())
	}

	#[test]
	fn plan_offset_inserts_kink_station() -> anyhow::Result<()> {
		let lower = opening_facing(Vec3::new(0.0, 0.0, -3.0), 1.0, 1.0, Vec2::Y)?;
		let upper = opening_facing(Vec3::new(3.0, 3.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		assert_eq!(well.polyline().stations.len(), 3);
		let mid = well.polyline().stations[1].center;
		assert!(mid.x.abs() < 1e-3 && mid.z.abs() < 1e-3, "mid={mid:?}");
		assert!((mid.y - 1.5).abs() < 1e-3, "mid.y={}", mid.y);
		Ok(())
	}

	#[test]
	fn run_in_has_floor_and_no_ceiling() -> anyhow::Result<()> {
		let lower = opening_facing(Vec3::new(-2.0, 0.0, 0.0), 1.0, 1.0, Vec2::X)?;
		let upper = opening_facing(Vec3::new(2.0, 3.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		assert!(well.run_in().faces().floor);
		assert!(!well.run_in().faces().ceiling);
		assert!(!well.run_in().floor().pieces().is_empty());
		assert_eq!(well.run_in().nodes().len(), 2);
		let inward =
			well.run_in().nodes()[1].bottom_middle - well.run_in().nodes()[0].bottom_middle;
		assert!(inward.x > 0.5, "run-in should follow +X orientation, got {inward:?}");
		Ok(())
	}

	#[test]
	fn fills_spiral_stairs_not_walls() -> anyhow::Result<()> {
		let lower = opening_facing(Vec3::new(0.0, 0.0, -3.0), 1.0, 1.0, Vec2::Y)?;
		let upper = opening_facing(Vec3::new(3.0, 3.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let stairs = well.stair_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(stairs.len(), 1);
		assert!(matches!(&stairs[0].geometry, Stair::Spiral(g) if g.height > 2.9));
		let wall_panels = well.panel_nodes_for_level(LodSceneLevel::High).flatten().len();
		// Run-in floor only — no side-wall tubes.
		assert!(wall_panels > 0);
		assert!(well.run_in().faces().left == false && well.run_in().faces().right == false);
		Ok(())
	}

	#[test]
	fn same_y_keeps_explicit_lower_as_anchor() -> anyhow::Result<()> {
		let a = opening_facing(Vec3::new(-2.0, 1.0, 0.0), 1.0, 1.0, Vec2::X)?;
		let b = opening_facing(Vec3::new(2.0, 1.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(a, b);
		assert!((well.lower().walk_on_mid().x + 2.0).abs() < 1e-3);
		assert!((well.upper().walk_on_mid().x - 2.0).abs() < 1e-3);
		assert!(well.polyline().rise().abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn walk_on_is_lower_edge() -> anyhow::Result<()> {
		let opening = opening_facing(Vec3::new(0.0, 1.5, 0.0), 1.0, 1.0, Vec2::X)?;
		let end = StairwellOpening::new(opening);
		assert!((end.walk_on_mid() - Vec3::new(0.0, 1.5, 0.0)).length() < 1e-3);
		assert!((end.headroom() - 2.0).abs() < 1e-3);
		Ok(())
	}
}
