//! Usage areas describe different spaces in a building.
//! They are usually concerned with things like subdividing rooms, adding furniture, etc.

pub mod clearance;
pub mod commercial_stall_strip;
pub mod common_bedroom;

pub use clearance::{PassageClearance, PlanHost, PASSAGE_CLEARANCE};
pub use commercial_stall_strip::commercial_stall::{
	BitesSitdownStall, BitesStall, CommercialStall, CommercialStallInterior,
	CommercialStallParameterized, CommercialStallPlan, KnickKnackStall, Lounge, MiniMart,
	PartsStall, PublicRestroom,
};
pub use commercial_stall_strip::{
	CommercialStallStrip, CommercialStallStripParameterized, CommercialStallStripPlan,
};
