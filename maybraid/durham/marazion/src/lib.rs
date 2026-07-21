//! Marazion watershed stamps ([RFC-127](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds)).
//!
//! Pure stamp / layout construction — no LOD `GenerationScheme` wiring.
//! Durham models consume [`jersey_terrain_stamps::JerseyModulation`] and
//! [`WaterFill`] outputs after pre-watershed terrain is composed.

pub mod fill;
pub mod lake;
pub mod noise;

pub use fill::WaterFill;
pub use lake::{Lake, LakeBandBudget, LakeParams};
