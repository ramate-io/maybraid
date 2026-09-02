//! Storey / stairwell circulation for movement intelligence.
//!
//! [`CirculationStorey`] and [`CirculationStairwell`] are world-space IR stamps.
//! [`RichmondAvianMovementSurface`] composes them with
//! [`movement_intelligence_avian::AvianMovementSurface`]: same storey stays collider
//! `MoveTo`s; a storey change prepends a stair polyline.

mod circulation;
mod surface;

pub use circulation::{
	circulation_from_stairwell, circulation_from_storey, CirculationStairwell, CirculationStorey,
};
pub use surface::RichmondAvianMovementSurface;
