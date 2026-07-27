//! Roof / cap scene components.
//!
//! Caps follow the same arc + struct filler idea as floors: stone for the spire
//! and perch shell, with occasional wood decking on the perch.

/// Conical (or faceted) spire roof above the central column.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneSpireRoof;

/// Wider perch roof / parapet cap for the top floor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonePerchRoof;

/// Occasional wood decking on the perch platform.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodPerchDeck;

crate::impl_empty_lod_scene!(RoughStoneSpireRoof, RoughStonePerchRoof, WoodPerchDeck);
