//! Vertical wedge placeholder (elevation kink). Empty until a kit GLB exists.

/// Fills the gap between upright linear partitions at a height change.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkWedge;

crate::impl_empty_lod_scene!(RoughStoneworkWedge);
