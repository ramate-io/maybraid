//! Rectangular-well spiral (quarter turns around a void) along a polyline.

use lod::gen::LodSceneLevel;
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::stair_flights::FlightPolyline;

/// Rectangular spiral flight. Tread / landing fill comes later.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularSpiralFlight {
	polyline: FlightPolyline,
}

impl RectangularSpiralFlight {
	pub fn new(polyline: FlightPolyline) -> Self {
		Self { polyline }
	}

	pub fn polyline(&self) -> &FlightPolyline {
		&self.polyline
	}
}

impl BuildingComponents for RectangularSpiralFlight {
	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::new()
	}
}
