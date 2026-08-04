//! Shared layout helpers for commercial stall interiors.
//!
//! - [`facade`] — cardinal [`StallSide`] bands (retail / office / restroom)
//! - [`bites`] — plan-face bites counters / seating / kitchen
//! - [`mini_mart`] — MiniMart clearances / office / register / aisles / shelves

pub mod bites;
pub mod facade;
pub mod mini_mart;

pub use bites::{
	BitesCounterChoice, BitesKitchen, BitesPassageSpec, BitesSitdownRegions, EligibleBitesPassage,
	PackedBitesCounters, BITES_COUNTER_ALONG_MIN, BITES_COUNTER_PLACE_RATE,
	BITES_PASSAGE_REMAIN_MIN, BITES_REGION_MIN_PLAN, BITES_SEATING_FACE_CONTACT,
};
pub use facade::{primary_facade, StallSide};
pub use mini_mart::{
	MiniMartPacked, MiniMartRegions, MiniMartShelfSpec, MINI_MART_AISLES_MIN,
	MINI_MART_DOOR_WIDTH_MAX, MINI_MART_DOOR_WIDTH_MIN, MINI_MART_OFFICE_LONG_MIN,
	MINI_MART_OFFICE_SHORT_MIN, MINI_MART_REGISTER_MIN, MINI_MART_SHELF_DEPTH_MAX,
	MINI_MART_SHELF_DEPTH_MIN, MINI_MART_SHELF_PLACE_RATE,
};

use bevy_math::bounding::Aabb3d;
use crate::paneling::Rectangle;

/// Prefer [`StallSide::facade_band`].
pub fn facade_band(bounds: &Aabb3d, side: StallSide, depth: f32, cover: f32) -> Aabb3d {
	side.facade_band(bounds, depth, cover)
}

/// Prefer [`StallSide::inset_band`].
pub fn inset_band(bounds: &Aabb3d, side: StallSide, offset: f32, depth: f32) -> Aabb3d {
	side.inset_band(bounds, offset, depth)
}

/// Prefer [`StallSide::back_third`].
pub fn back_third(bounds: &Aabb3d, side: StallSide) -> Aabb3d {
	side.back_third(bounds)
}

/// Prefer [`StallSide::office_divider_wall`].
pub fn office_divider_wall(bounds: &Aabb3d, office: &Aabb3d, side: StallSide) -> Option<Rectangle> {
	side.office_divider_wall(bounds, office)
}

/// Prefer [`StallSide::sales_minus_office`].
pub fn sales_minus_office(bounds: &Aabb3d, office: &Aabb3d, side: StallSide) -> Aabb3d {
	side.sales_minus_office(bounds, office)
}
