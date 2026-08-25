//! Stair flight fillers over a connecting polyline.
//!
//! A [`FlightPolyline`] describes center and clear headroom per station. Rise is
//! \(\Delta Y\) between stations. Families compose existing [`StairNode`]s and
//! landing panels; they do not add new [`richmond_building_components::stairs::StairGeometry`]
//! variants. A later `StairCollection` can merge many treads under one LOD parent.

pub mod rectangular_spiral;
pub mod run_and_landing;
pub mod spiral;

pub use rectangular_spiral::RectangularSpiralFlight;
pub use run_and_landing::RunAndLandingFlight;
pub use spiral::{SpiralFlight, SpiralFlightFit};

use bevy_math::Vec3;

/// One station along a flight centerline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightStation {
	pub center: Vec3,
	/// Clear headroom at this station (meters).
	pub height: f32,
}

/// Centerline the stairwell invents; flights fill it.
#[derive(Debug, Clone, PartialEq)]
pub struct FlightPolyline {
	pub stations: Vec<FlightStation>,
}

impl FlightPolyline {
	pub fn new(stations: impl IntoIterator<Item = FlightStation>) -> Self {
		Self { stations: stations.into_iter().collect() }
	}

	/// Absolute walk-on \(Y\) span between the first and last stations.
	pub fn rise(&self) -> f32 {
		match (self.stations.first(), self.stations.last()) {
			(Some(a), Some(b)) => (b.center.y - a.center.y).abs(),
			_ => 0.0,
		}
	}
}
