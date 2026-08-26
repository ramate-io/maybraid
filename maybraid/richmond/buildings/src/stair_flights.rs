//! Stair flight fillers over a connecting polyline.
//!
//! A [`FlightPolyline`] describes center and clear headroom per station. Rise is
//! \(\Delta Y\) between stations. Families compose [`StraightStair`] nodes and
//! landing panels; they do not add [`richmond_building_components::stairs::StairGeometry`]
//! variants. A circular spiral is many one-tread straight nodes around a center.

pub(crate) mod composed;
pub(crate) mod geom;
pub mod rectangular_spiral;
pub mod run_and_landing;
pub mod spiral;
pub mod tread_end;

pub use composed::ComposedFlight;
pub use rectangular_spiral::fit as fit_rectangular_spiral;
pub use run_and_landing::fit as fit_run_and_landing;
pub use spiral::{circular_straight_nodes, fit as fit_spiral};
pub use tread_end::TreadEnd;

/// Family structs collapsed onto [`ComposedFlight`].
pub type SpiralFlight = ComposedFlight;
pub type RectangularSpiralFlight = ComposedFlight;
pub type RunAndLandingFlight = ComposedFlight;
pub type SpiralFlightFit = WellFit;

use bevy_math::Vec2;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{BuildingComponents, Layers};

/// Inputs for fitting a flight inside a vertical shaft (two horizontal faces).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WellFit {
	pub lower_center: Vec3,
	pub upper_center: Vec3,
	pub lower_walk_on: Vec3,
	pub upper_walk_on: Vec3,
	/// XZ walk-off from the lower walk-on into the well.
	pub lower_out: Vec2,
	pub lower_half_width: f32,
	pub lower_half_depth: f32,
	pub upper_half_width: f32,
	pub upper_half_depth: f32,
	/// Tread span as a fraction of the tighter opening half-extent.
	pub tread_fill: f32,
	/// Lapping ratio (preferred going / width) — not “how chunky one tread looks.”
	/// High values add rectangular-spiral circuits or side-by-side runs.
	pub lapping_ratio: f32,
}

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
pub struct StairwellFlight {
	kind: StairwellFlightKind,
	composed: ComposedFlight,
}

impl StairwellFlight {
	pub fn fit(
		kind: StairwellFlightKind,
		polyline: FlightPolyline,
		fit: WellFit,
		style: PanelStyle,
		slab_thickness: f32,
	) -> Self {
		let composed = match kind {
			StairwellFlightKind::Spiral => spiral::fit(polyline, fit),
			StairwellFlightKind::RectangularSpiral => {
				rectangular_spiral::fit(polyline, fit, style, slab_thickness)
			}
			StairwellFlightKind::RunAndLanding => {
				run_and_landing::fit(polyline, fit, style, slab_thickness)
			}
		};
		Self { kind, composed }
	}

	pub fn kind(&self) -> StairwellFlightKind {
		self.kind
	}

	pub fn composed(&self) -> &ComposedFlight {
		&self.composed
	}

	pub fn polyline(&self) -> &FlightPolyline {
		self.composed.polyline()
	}

	pub fn last_tread_xz(&self) -> Vec2 {
		self.composed.last_tread_xz()
	}

	pub fn last_tread_travel_xz(&self) -> Vec2 {
		self.composed.last_tread_travel_xz()
	}

	pub fn last_tread_leading_xz(&self) -> (Vec2, Vec2) {
		self.composed.last_tread_leading_xz()
	}

	pub fn tread_end(&self) -> TreadEnd {
		self.composed.tread_end().unwrap_or(TreadEnd {
			leading_outer: Vec2::ZERO,
			leading_inner: Vec2::ZERO,
			travel: Vec2::X,
		})
	}
}

impl BuildingComponents for StairwellFlight {
	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		self.composed.stair_nodes_for_level(level)
	}

	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.composed.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.composed.joint_nodes_for_level(level)
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
