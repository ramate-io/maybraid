//! Les Halles storey.
//!
//! The basic Les Halles layout is an outer ring gallery with an inner ring balcony.
//! The outer ring gallery is meant to reflect commercial space,
//! while the inner ring balcony is the walking access to the commercial space.
//!
//! The Les Halles floor plan...
//! - Uses the full rectangular width and height of the confines.
//! - Decides reasonable widths for the gallery and balcony.
//! - Adds reasonable additional doors and windows in the open set along the gallery.
//! - Chooses whether to allocate shafts (for stairs) in the four corners or in the middle of each side.

pub mod floor_plan;
pub mod full_storey;
pub mod parameterized;

pub use floor_plan::LesHallesFloorPlan;
pub use full_storey::LesHallesFullStorey;
pub use parameterized::{LesHallesParameterized, LesHallesShaftPlacement};

/// Scope prefix for [`crate::OpeningId::scoped`] openings authored by this typology.
pub const SCOPE: &str = "les_halles";
