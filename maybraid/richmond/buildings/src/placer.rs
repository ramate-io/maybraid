//! Predicate-based rectangular layout trier.
//!
//! Callers own a catalog of [`kind::KindSpec`] rows (tier, propose knobs,
//! predicates, caps). The geometric loop is domain-agnostic: pick → propose →
//! try predicates → commit. Priority lives in kind selection
//! ([`tiers`]), not in separate packer passes. Suitable for usage areas,
//! stall interiors, and other AABB-in-host packing.

pub mod budget;
pub mod furniture;
pub mod kind;
pub mod predicates;
pub mod tiers;
pub mod walled_room;

pub use budget::OccupiedBudget;
pub use furniture::{try_free_extent, try_wall_long, FreeExtentKnobs, WallLongKnobs};
pub use kind::{CommitEffect, KindSpec, Predicate, ProposeKnobs, SoftGoalRole};
pub use predicates::{
	against_wall, approach_free, clear_of_keep_outs, in_host, long_face_on_wall, PredicateCtx,
};
pub use tiers::{enclosure_soft_goal_met, pick_kind, soft_goal_credit, ProgramTier};
pub use walled_room::WalledRoomFill;
