//! Usage areas describe different spaces in a building.
//! They are usually concerned with things like subdividing rooms, adding furniture, etc.

pub mod commercial_stall;
pub mod commercial_stall_strip;
pub mod common_bedroom;

pub use commercial_stall::{
	CommercialStall, CommercialStallParameterized, CommercialStallPlan,
};
pub use commercial_stall_strip::{
	CommercialStallStrip, CommercialStallStripParameterized, CommercialStallStripPlan,
};
