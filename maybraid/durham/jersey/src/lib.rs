//! Jersey terrain stamps ([RFC-105 §3.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain)).
//!
//! Pure stamp / modulation construction — no LOD `GenerationScheme` wiring.
//! Durham models consume [`JerseyModulation`] and stamp outputs.

pub mod modulation;
pub mod region;
pub mod stamp;
pub mod stamps;

pub use modulation::{JerseyModulation, RegionAffineModulation, RegionGradingModulation};
pub use region::{CircleRegion, RectRegion, Region2D, RegionNoise};
pub use stamp::{StampSemantics, StampSet};
pub use stamps::{
	ValleyBasin, ValleyBasinParams, ValleyCrossSection, ValleyFloorKind,
};
