//! One flight: polyline + straight stairs + level pads.

use bevy_math::Vec2;
use lod::gen::LodSceneLevel;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::quad_panel::QuadPanel;
use crate::stair_flights::geom::xz;
use crate::stair_flights::{FlightPolyline, TreadEnd};

/// Fitted stairs and rest pads over a [`FlightPolyline`].
#[derive(Debug, Clone, PartialEq)]
pub struct ComposedFlight {
	polyline: FlightPolyline,
	stairs: Vec<StairNode>,
	pads: Vec<QuadPanel>,
}

impl ComposedFlight {
	pub fn new(polyline: FlightPolyline, stairs: Vec<StairNode>, pads: Vec<QuadPanel>) -> Self {
		Self { polyline, stairs, pads }
	}

	pub fn polyline(&self) -> &FlightPolyline {
		&self.polyline
	}

	pub fn stairs(&self) -> &[StairNode] {
		&self.stairs
	}

	pub fn pads(&self) -> &[QuadPanel] {
		&self.pads
	}

	pub fn tread_end(&self) -> Option<TreadEnd> {
		TreadEnd::from_last_straight(&self.stairs)
	}

	/// Last-tread plan point just behind the leading edge.
	pub fn last_tread_xz(&self) -> Vec2 {
		self.tread_end()
			.map(|e| e.leading_mid() - e.travel * 0.01)
			.or_else(|| self.stairs.last().map(|n| xz(n.placement.translation)))
			.unwrap_or(Vec2::ZERO)
	}

	pub fn last_tread_travel_xz(&self) -> Vec2 {
		self.tread_end().map(|e| e.travel).unwrap_or(Vec2::X)
	}

	pub fn last_tread_leading_xz(&self) -> (Vec2, Vec2) {
		self.tread_end().map(|e| (e.leading_outer, e.leading_inner)).unwrap_or_else(|| {
			let p = self.last_tread_xz();
			(p, p)
		})
	}
}

impl BuildingComponents for ComposedFlight {
	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::from_free(self.stairs.clone())
	}

	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for pad in &self.pads {
			out.extend(pad.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for pad in &self.pads {
			out.extend(pad.joint_nodes_for_level(level));
		}
		out
	}

	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::new()
	}
}
