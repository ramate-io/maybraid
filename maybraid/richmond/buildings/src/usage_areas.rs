//! Usage areas: fill residual [`crate::fit::Confines`] with program (rooms, shops).
//!
//! Higher-order storeys (e.g. Les Halles) emit shell geometry +
//! [`FillableRegions`]; usage areas consume those confines — packing furniture /
//! subdivisions / commercial interiors — and may emit nested residuals
//! (offices, toilet stalls, …). Shared helpers such as [`clearance`] and
//! [`enclosed_room`] live here so multiple usage areas can share passage
//! keep-outs and wall-seeded private rooms.

pub mod clearance;
pub mod commercial_stall_strip;
pub mod common_bedroom;
pub mod enclosed_room;
pub mod label_util;

pub use clearance::{
	abuts_clearance, max_empty_abutting_clearance, max_empty_abutting_clearance_sized,
	pack_abutting_clearance, PassageClearance, PlanHost, PASSAGE_CLEARANCE,
};
pub use commercial_stall_strip::commercial_stall::{
	BitesSitdownStall, BitesStall, CommercialStall, CommercialStallInterior,
	CommercialStallParameterized, CommercialStallPlan, KnickKnackStall,
	KnickKnackStallParameterized, KnickKnackStallPlan, Lounge, MiniMart, PartsStall, PublicRestroom,
	PublicRestroomParameterized, PublicRestroomPlan,
};
pub use commercial_stall_strip::{
	CommercialStallStrip, CommercialStallStripParameterized, CommercialStallStripPlan,
};
pub use common_bedroom::{
	CommonBedroom, CommonBedroomParameterized, CommonBedroomPlan, SCOPE as COMMON_BEDROOM_SCOPE,
};
pub use enclosed_room::{EnclosedRoom, EnclosedRoomMins, EnclosedRoomParams};
