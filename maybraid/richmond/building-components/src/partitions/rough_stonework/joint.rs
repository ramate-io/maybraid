//! Horizontal joint placeholder (plan-angle vertex). Empty until a kit GLB exists.

/// Circular / post joint between upright linear partition segments.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkJoint;

crate::impl_empty_lod_scene!(RoughStoneworkJoint);
