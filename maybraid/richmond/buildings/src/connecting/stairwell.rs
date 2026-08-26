//! Two-opening stairwell: horizontal shaft faces → slabs + fitted stair flight.
//!
//! Each [`StairwellOpening`] is a **horizontal** [`MappedOpening`] (shaft
//! cross-section). The pair describes a vertical well: `lower` is the floor-space
//! anchor, `upper` is the open top. On each quad the **lower** edge is the
//! walk-on; the **upper** edge is the far side of the hole. `orientation` is XZ
//! walk-off from that walk-on into the well.
//!
//! Owned floors are thin [`QuadPanel`] slabs at [`SLAB_THICKNESS_M`] (run-in at
//! the lower walk-on; optional upper landing flush with the last tread, along
//! the nearby rim). Kit top sits on that plane — rectangle / triangle panels
//! are centered on local \(Y = 0\) with \(\pm 0.2\,\mathrm{m}\). Turn the
//! landing off when a follow-on stairwell will own that floor. The shaft is
//! filled with composed [`StairNode`]s. It does not author walls or emit shaft
//! opening labels. A [`FlightPolyline`] along face centers absorbs plan offset.
//! Choose a family with [`Self::with_flight`].

mod landing;
mod opening;
mod tread;

pub use opening::StairwellOpening;
pub use tread::TreadEnd;

use lod::gen::LodSceneLevel;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::quad_panel::QuadPanel;
use crate::stair_flights::{FlightPolyline, StairwellFlight, StairwellFlightKind};

/// Aesthetic run-in depth / upper-landing length along the rim (meters).
pub const RUN_IN_M: f32 = 0.75;

/// Kit thickness for both owned floor slabs (meters).
pub const SLAB_THICKNESS_M: f32 = 0.05;

/// Shortest authored slab along the rim (meters).
const MIN_SLAB_M: f32 = 0.12;

/// Two horizontal shaft faces → run-in / optional upper landing + stair flight.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectingStairwell {
	style: PanelStyle,
	lower: StairwellOpening,
	upper: StairwellOpening,
	polyline: FlightPolyline,
	run_in: QuadPanel,
	want_landing: bool,
	slab_thickness: f32,
	upper_landing: Option<QuadPanel>,
	kind: StairwellFlightKind,
	flight: StairwellFlight,
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
		let slab_thickness = SLAB_THICKNESS_M;
		let kind = StairwellFlightKind::Spiral;
		let flight = lower.flight_to(upper, kind, style, slab_thickness);
		let polyline = flight.polyline().clone();
		let run_in = lower.run_in_slab(style, slab_thickness);
		let upper_landing = flight.landing_slab(upper, style, slab_thickness);
		Self {
			style,
			lower,
			upper,
			polyline,
			run_in,
			want_landing: true,
			slab_thickness,
			upper_landing,
			kind,
			flight,
		}
	}

	pub fn rough_stone(
		lower: impl Into<StairwellOpening>,
		upper: impl Into<StairwellOpening>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, lower, upper)
	}

	/// Upper landing flush with the last tread, on the opening rim.
	///
	/// Default is `true`. Set `false` when a follow-on stairwell will own that
	/// landing (its lower run-in).
	pub fn with_upper_landing(mut self, enabled: bool) -> Self {
		self.want_landing = enabled;
		self.rebuild_slabs();
		self
	}

	/// Kit thickness of both owned slabs (meters). Default [`SLAB_THICKNESS_M`].
	pub fn with_slab_thickness(mut self, thickness: f32) -> Self {
		self.slab_thickness = thickness.max(1e-4);
		self.rebuild_flight();
		self
	}

	/// Replace the shaft fill. Default is [`StairwellFlightKind::Spiral`].
	pub fn with_flight(mut self, kind: StairwellFlightKind) -> Self {
		self.kind = kind;
		self.rebuild_flight();
		self
	}

	pub fn slab_thickness(&self) -> f32 {
		self.slab_thickness
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.rebuild_slabs();
		self.run_in = self.run_in.clone().with_joint_policy(joint_policy);
		self.upper_landing =
			self.upper_landing.take().map(|slab| slab.with_joint_policy(joint_policy));
		self
	}

	fn rebuild_flight(&mut self) {
		self.flight =
			self.lower.flight_to(self.upper, self.kind, self.style, self.slab_thickness);
		self.polyline = self.flight.polyline().clone();
		self.rebuild_slabs();
	}

	fn rebuild_slabs(&mut self) {
		self.run_in = self.lower.run_in_slab(self.style, self.slab_thickness);
		self.upper_landing = self
			.want_landing
			.then(|| self.flight.landing_slab(self.upper, self.style, self.slab_thickness))
			.flatten();
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

	pub fn run_in(&self) -> &QuadPanel {
		&self.run_in
	}

	pub fn upper_landing(&self) -> Option<&QuadPanel> {
		self.upper_landing.as_ref()
	}

	pub fn flight(&self) -> &StairwellFlight {
		&self.flight
	}

	pub fn flight_kind(&self) -> StairwellFlightKind {
		self.kind
	}
}

impl BuildingComponents for ConnectingStairwell {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.run_in.panel_nodes_for_level(level);
		if let Some(landing) = &self.upper_landing {
			out.extend(landing.panel_nodes_for_level(level));
		}
		out.extend(self.flight.panel_nodes_for_level(level));
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.run_in.joint_nodes_for_level(level);
		if let Some(landing) = &self.upper_landing {
			out.extend(landing.joint_nodes_for_level(level));
		}
		out.extend(self.flight.joint_nodes_for_level(level));
		out
	}

	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::new()
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		self.flight.stair_nodes_for_level(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connecting::geom::normalize_xz;
	use crate::openings::MappedOpening;
	use bevy_math::{Vec2, Vec3};
	use crate::stair_flights::StairwellFlightKind;
	use richmond_building_components::partitions::PANEL_Y_HALF;
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
		assert!((well.run_in().thickness() - SLAB_THICKNESS_M).abs() < 1e-3);
		let [a0, a1, b0, b1] = well.run_in().corners();
		assert!(
			(a0.y - (-PANEL_Y_HALF)).abs() < 1e-3,
			"run-in kit center should sit {PANEL_Y_HALF} below the walk-on, got y={}",
			a0.y
		);
		let inward = (b0 + b1) * 0.5 - (a0 + a1) * 0.5;
		assert!(inward.x > 0.5, "run-in should follow +X into the shaft, got {inward:?}");
		Ok(())
	}

	#[test]
	fn upper_landing_follows_last_tread_travel() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, -Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let landing = well.upper_landing().expect("upper landing on by default");
		assert!((landing.thickness() - SLAB_THICKNESS_M).abs() < 1e-3);
		let [a, b, inner_start, ..] = landing.corners();
		assert!(
			(a.y - (3.0 - PANEL_Y_HALF)).abs() < 1e-3,
			"landing kit center should sit {PANEL_Y_HALF} below the walk-on, got y={}",
			a.y
		);
		for p in [a, b] {
			assert!(
				p.x.abs() <= 1.2 + 1e-3 && p.z.abs() <= 1.2 + 1e-3,
				"landing must stay in the opening, {p:?}"
			);
			assert!(
				well.upper().rim_distance(Vec2::new(p.x, p.z)) < 0.04,
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
		let start_edge = Vec2::new(inner_start.x - a.x, inner_start.z - a.z);
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
	fn slab_thickness_applies_to_both_floors() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper).with_slab_thickness(0.12);
		assert!((well.slab_thickness() - 0.12).abs() < 1e-4);
		assert!((well.run_in().thickness() - 0.12).abs() < 1e-4);
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
		assert!(
			with_n > without_n,
			"upper landing should add floor panels ({with_n} vs {without_n})"
		);
		Ok(())
	}

	#[test]
	fn fills_spiral_inside_shaft() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let stairs = well.stair_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!stairs.is_empty());
		assert!(stairs.iter().all(|s| matches!(s.geometry, Stair::Straight(_))));
		for s in &stairs {
			let p = s.placement.translation;
			assert!(
				p.x.abs() < 1.4 && p.z.abs() < 1.4,
				"spiral treads should sit in the shaft, got {p:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn rectangular_spiral_pads_corners_and_reaches_walk_on() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper)
			.with_flight(StairwellFlightKind::RectangularSpiral);
		let n = well.panel_nodes_for_level(LodSceneLevel::High).flatten().len();
		assert!(n > 2, "corner pads should add panels beyond run-in + upper landing, got {n}");
		let landing = well.upper_landing().expect("upper landing");
		let walk = well.upper().walk_on_mid();
		let reach = landing
			.corners()
			.into_iter()
			.map(|p| (Vec2::new(p.x, p.z) - Vec2::new(walk.x, walk.z)).length())
			.fold(f32::MAX, f32::min);
		assert!(reach < 0.25, "upper landing should reach the walk-on, nearest={reach}");
		Ok(())
	}

	#[test]
	fn with_flight_switches_family() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let rect = ConnectingStairwell::rough_stone(lower, upper)
			.with_flight(StairwellFlightKind::RectangularSpiral);
		assert_eq!(rect.flight_kind(), StairwellFlightKind::RectangularSpiral);
		assert!(rect.stair_nodes_for_level(LodSceneLevel::High).flatten().len() >= 2);
		let runs = ConnectingStairwell::rough_stone(lower, upper)
			.with_flight(StairwellFlightKind::RunAndLanding);
		assert_eq!(runs.flight_kind(), StairwellFlightKind::RunAndLanding);
		assert!(!runs.stair_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
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
