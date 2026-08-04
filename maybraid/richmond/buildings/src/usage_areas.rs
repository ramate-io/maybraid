//! Usage areas describe different spaces in a building.
//! They are usually concerned with things like subdividing rooms, adding furniture, etc.

pub mod commercial_stall_strip;
pub mod common_bedroom;

pub use commercial_stall_strip::commercial_stall::{
	BitesSitdownStall, BitesStall, CommercialStall, CommercialStallInterior,
	CommercialStallParameterized, CommercialStallPlan, KnickKnackStall, Lounge, PartsStall,
	MiniMart, PublicRestroom,
};
pub use commercial_stall_strip::{
	CommercialStallStrip, CommercialStallStripParameterized, CommercialStallStripPlan,
};
