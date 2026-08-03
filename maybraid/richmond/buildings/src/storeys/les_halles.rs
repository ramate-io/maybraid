//! Les Halles storey.
//!
//! The basic Les Halles layout is an outer ring gallery with an inner ring balcony.
//! The outer ring gallery is commercial space: a rectangular ring with outer facade
//! walls, inner walls facing the balcony, and a floor (ceiling optional / off by default).
//! The inner balcony ring is walking access: floor only, no walls, open to the courtyard.
//!
//! The Les Halles floor plan...
//! - Uses the full rectangular width and height of the confines.
//! - Decides reasonable widths for the gallery and balcony.
//! - Adds apertures on the outer facade and stall doors on the gallery’s inner wall.
//! - Chooses whether to allocate shafts (for stairs) in the four corners or in the middle of each side.

pub mod floor_plan;
pub mod full_storey;
pub mod parameterized;

pub use floor_plan::LesHallesFloorPlan;
pub use full_storey::LesHallesFullStorey;
pub use parameterized::{
	LesHallesParameterized, LesHallesPlacedDoor, LesHallesShaftPlacement, LesHallesStallDoor,
};

/// Scope prefix for [`crate::OpeningId::scoped`] openings authored by this typology.
pub const SCOPE: &str = "les_halles";
