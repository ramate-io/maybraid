//! Straight runs and landings along a polyline (L / U / offset as the path dictates).

use lod::gen::LodSceneLevel;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::stair_flights::FlightPolyline;

/// Run-and-landing flight. Tread and landing-panel fill comes later.
#[derive(Debug, Clone, PartialEq)]
pub struct RunAndLandingFlight {
	polyline: FlightPolyline,
}

impl RunAndLandingFlight {
	pub fn new(polyline: FlightPolyline) -> Self {
		Self { polyline }
	}

	pub fn polyline(&self) -> &FlightPolyline {
		&self.polyline
	}
}

impl BuildingComponents for RunAndLandingFlight {
	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::new()
	}

	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::new()
	}
}
