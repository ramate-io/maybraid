//! Usage areas: fill residual [`crate::fit::Confines`] with program (rooms, shops).
//!
//! Higher-order storeys (e.g. Les Halles) emit shell geometry +
//! [`FillableRegions`]; usage areas consume those confines — packing furniture /
//! subdivisions / commercial interiors — and may emit nested residuals
//! (offices, toilet stalls, …). Shared helpers such as [`clearance`] live here
//! so multiple usage areas can share passage keep-outs.

pub mod apartment;
pub mod clearance;
pub mod commercial_stall_strip;
pub mod common_bedroom;
pub mod janitorial;
pub mod plan_cells;

pub use apartment::{Apartment, ApartmentPiece};
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
pub use janitorial::Janitorial;
pub use plan_cells::{
	cell_has_hall_frontage, cells_edge_adjacent, group_cells_to_apartments, split_toward_min_room,
	subtract_aabb2, PlanCell,
};
