//! Stair flight fillers over a connecting polyline.
//!
//! A [`FlightPolyline`] describes center and clear headroom per station. Rise is
//! \(\Delta Y\) between stations. Families compose [`StraightStair`] nodes and
//! landing panels; they do not add [`richmond_building_components::stairs::StairGeometry`]
//! variants. A circular spiral is many one-tread straight nodes around a center.

pub(crate) mod geom;
pub mod rectangular_spiral;
pub mod run_and_landing;
pub mod spiral;
pub mod tread_end;

pub use rectangular_spiral::RectangularSpiralFlight;
pub use run_and_landing::RunAndLandingFlight;
pub use spiral::{circular_straight_nodes, SpiralFlight, SpiralFlightFit};
pub use tread_end::TreadEnd;

use bevy_math::Vec2;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{BuildingComponents, Layers};

/// Which family [`crate::ConnectingStairwell::with_flight`] should fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StairwellFlightKind {
	#[default]
	Spiral,
	RectangularSpiral,
	RunAndLanding,
}

/// Fitted flight inside a connecting stairwell.
#[derive(Debug, Clone, PartialEq)]
pub enum StairwellFlight {
	Spiral(SpiralFlight),
	RectangularSpiral(RectangularSpiralFlight),
	RunAndLanding(RunAndLandingFlight),
}

impl StairwellFlight {
	pub fn fit(
		kind: StairwellFlightKind,
		polyline: FlightPolyline,
		fit: SpiralFlightFit,
		style: PanelStyle,
		slab_thickness: f32,
	) -> Self {
		match kind {
			StairwellFlightKind::Spiral => Self::Spiral(SpiralFlight::fit(polyline, fit)),
			StairwellFlightKind::RectangularSpiral => {
				Self::RectangularSpiral(RectangularSpiralFlight::fit(
					polyline,
					fit,
					style,
					slab_thickness,
				))
			}
			StairwellFlightKind::RunAndLanding => {
				Self::RunAndLanding(RunAndLandingFlight::fit(polyline, fit, style, slab_thickness))
			}
		}
	}

	pub fn kind(&self) -> StairwellFlightKind {
		match self {
			Self::Spiral(_) => StairwellFlightKind::Spiral,
			Self::RectangularSpiral(_) => StairwellFlightKind::RectangularSpiral,
			Self::RunAndLanding(_) => StairwellFlightKind::RunAndLanding,
		}
	}

	pub fn polyline(&self) -> &FlightPolyline {
		match self {
			Self::Spiral(f) => f.polyline(),
			Self::RectangularSpiral(f) => f.polyline(),
			Self::RunAndLanding(f) => f.polyline(),
		}
	}

	pub fn last_tread_xz(&self) -> Vec2 {
		match self {
			Self::Spiral(f) => f.last_tread_xz(),
			Self::RectangularSpiral(f) => f.last_tread_xz(),
			Self::RunAndLanding(f) => f.last_tread_xz(),
		}
	}

	pub fn last_tread_travel_xz(&self) -> Vec2 {
		match self {
			Self::Spiral(f) => f.last_tread_travel_xz(),
			Self::RectangularSpiral(f) => f.last_tread_travel_xz(),
			Self::RunAndLanding(f) => f.last_tread_travel_xz(),
		}
	}

	pub fn last_tread_leading_xz(&self) -> (Vec2, Vec2) {
		match self {
			Self::Spiral(f) => f.last_tread_leading_xz(),
			Self::RectangularSpiral(f) => f.last_tread_leading_xz(),
			Self::RunAndLanding(f) => f.last_tread_leading_xz(),
		}
	}

	pub fn tread_end(&self) -> TreadEnd {
		TreadEnd {
			leading_outer: self.last_tread_leading_xz().0,
			leading_inner: self.last_tread_leading_xz().1,
			travel: self.last_tread_travel_xz(),
		}
	}
}

impl BuildingComponents for StairwellFlight {
	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		match self {
			Self::Spiral(f) => f.stair_nodes_for_level(level),
			Self::RectangularSpiral(f) => f.stair_nodes_for_level(level),
			Self::RunAndLanding(f) => f.stair_nodes_for_level(level),
		}
	}

	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::RectangularSpiral(f) => f.panel_nodes_for_level(level),
			Self::RunAndLanding(f) => f.panel_nodes_for_level(level),
			_ => Layers::new(),
		}
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		match self {
			Self::RectangularSpiral(f) => f.joint_nodes_for_level(level),
			Self::RunAndLanding(f) => f.joint_nodes_for_level(level),
			_ => Layers::new(),
		}
	}
}

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

