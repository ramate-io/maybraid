//! I-Apartment storey.
//!
//! Fit an [`crate::IFloor`] I / T / U / L / Z envelope to confines from seed, then
//! expose the shell’s natural **1–3 primary rectangles** (stem + optional flanges).
//!
//! Pipeline:
//! - [`IApartmentParameterized::sample`] — I-frame layout knobs
//! - [`IApartmentFloorPlan::from_parameterized`] — shell + primary rect regions
//! - [`IApartmentFullStorey::from_floor_plan`] — one [`crate::LivableApartment`] per rect

pub mod floor_plan;
pub mod full_storey;
pub mod parameterized;

pub use floor_plan::IApartmentFloorPlan;
pub use full_storey::IApartmentFullStorey;
pub use parameterized::IApartmentParameterized;

/// Scope prefix for [`crate::OpeningId::scoped`] openings authored by this typology.
pub const SCOPE: &str = "i_apartment";
