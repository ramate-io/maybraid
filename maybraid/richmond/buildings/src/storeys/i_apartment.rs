//! I-Apartment storey.
//!
//! An I / T / U / L / Z envelope ([`crate::IFloor`]) is packed with hallway spines,
//! boundary shaft pockets, janitorial closets, and multi-cell apartments formed by
//! grouping residual plan cells that touch a hallway.
//!
//! Pipeline:
//! - [`IApartmentParameterized::sample`] — hall / room / apartment knobs
//! - [`IApartmentFloorPlan::from_parameterized`] — shell + halls + shafts + groups
//! - [`IApartmentFullStorey::from_floor_plan`] — [`crate::Apartment`] + [`crate::Janitorial`]

pub mod floor_plan;
pub mod full_storey;
pub mod parameterized;

pub use floor_plan::{ApartmentGroup, IApartmentFloorPlan};
pub use full_storey::IApartmentFullStorey;
pub use parameterized::IApartmentParameterized;

/// Scope prefix for [`crate::OpeningId::scoped`] openings authored by this typology.
pub const SCOPE: &str = "i_apartment";
