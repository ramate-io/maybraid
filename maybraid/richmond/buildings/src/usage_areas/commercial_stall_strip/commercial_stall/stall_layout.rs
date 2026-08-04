//! Shared layout helpers for commercial stall interiors.
//!
//! Passage clearance lives in [`crate::usage_areas::clearance`] (usage-area-wide).
//!
//! - [`facade`] — cardinal [`StallSide`] bands (retail / office / restroom)
//! - [`bites`] — plan-face bites counters / seating / kitchen
//! - [`mini_mart`] — MiniMart clearances / office / register / aisles / shelves
//! - [`parts`] — Parts office / parts pockets
//!
//! Domain constants live in the submodule that owns them; this root re-exports
//! types used across interior modules.

pub mod bites;
pub mod facade;
pub mod mini_mart;
pub mod parts;

pub use bites::{
	BitesCounterChoice, BitesKitchen, BitesPassageSpec, BitesSitdownRegions, EligibleBitesPassage,
	PackedBitesCounters,
};
pub use facade::{primary_facade, StallSide};
pub use mini_mart::{MiniMartPacked, MiniMartRegions, MiniMartShelfSpec};

use bevy_math::bounding::Aabb3d;

/// Prefer [`StallSide::facade_band`].
pub fn facade_band(bounds: &Aabb3d, side: StallSide, depth: f32, cover: f32) -> Aabb3d {
	side.facade_band(bounds, depth, cover)
}

/// Prefer [`StallSide::inset_band`].
pub fn inset_band(bounds: &Aabb3d, side: StallSide, offset: f32, depth: f32) -> Aabb3d {
	side.inset_band(bounds, offset, depth)
}
