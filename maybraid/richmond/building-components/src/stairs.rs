//! Circulation stair scene components.
//!
//! Spiral runs are stone-primary (spire core). Straight runs may be stone or wood.

/// Spiral stair occupying the exclusive spire rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneSpiralStair;

/// Straight stone stair between floor levels or room halfspaces.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneStraightStair;

/// Occasional wood straight stair for interior halfspaces.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodStraightStair;

crate::impl_empty_lod_scene!(
	RoughStoneSpiralStair,
	RoughStoneStraightStair,
	WoodStraightStair,
);
