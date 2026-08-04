//! Usage areas: fill residual [`crate::fit::Confines`] with program (rooms, shops).
//!
//! Higher-order storeys (e.g. Les Halles) emit shell geometry +
//! [`FillableRegions`]; usage areas consume those confines — packing furniture /
//! subdivisions / commercial interiors — and may emit nested residuals
//! (offices, toilet stalls, …). Shared helpers such as [`clearance`] and
//! [`enclosed_room`] live here for passage keep-outs and wall-seeded private
//! rooms. Rectangular place-and-commit packing uses the crate-root
//! [`crate::placer`] KindSpec trier.

pub mod clearance;
pub mod commercial_stall_strip;
pub mod common_bedroom;
pub mod enclosed_room;
pub mod enclosure_panels;
pub mod furniture_util;
pub mod label_util;
pub mod livable_quarters;

pub use clearance::{
	abuts_clearance, approach_blocked, approach_zone, commit_door_clear,
	max_empty_abutting_clearance, max_empty_abutting_clearance_sized, pack_abutting_clearance,
	PassageClearance, PlanHost, PASSAGE_APPROACH_PAD, PASSAGE_CLEARANCE,
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
pub use livable_quarters::{
	DiningRoom, DiningRoomParameterized, DiningRoomPlan, Kitchen, KitchenParameterized, KitchenPlan,
	LivingRoom, LivingRoomParameterized, LivingRoomPlan, ResidentialBathroom,
	ResidentialBathroomParameterized, ResidentialBathroomPlan, ResidentialHalfBathroom,
	ResidentialHalfBathroomParameterized, ResidentialHalfBathroomPlan, SittingRoom,
	SittingRoomParameterized, SittingRoomPlan, Study, StudyParameterized, StudyPlan,
	DINING_ROOM_SCOPE, KITCHEN_SCOPE, LIVING_ROOM_SCOPE, RESIDENTIAL_BATHROOM_SCOPE,
	RESIDENTIAL_HALF_BATHROOM_SCOPE, SITTING_ROOM_SCOPE, STUDY_SCOPE,
};
