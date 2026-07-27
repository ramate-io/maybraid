//! Floor slab scene components.
//!
//! Floors are typically an **arc filler** (curved disc segments) plus a
//! **struct filler** (radial/rect bracing). Prefer rough stonework; wood is
//! occasional for interior halfspaces.

/// Arc-segment floor fill in rough stone (for circular tower discs).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneFloorArcFill;

/// Structural / radial floor bracing in rough stone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneFloorStructFill;

/// Occasional wood arc floor fill for interior rooms.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodFloorArcFill;

/// Occasional wood structural floor bracing.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodFloorStructFill;

crate::impl_empty_lod_scene!(
	RoughStoneFloorArcFill,
	RoughStoneFloorStructFill,
	WoodFloorArcFill,
	WoodFloorStructFill,
);
