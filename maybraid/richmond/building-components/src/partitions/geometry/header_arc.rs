//! Header-arc helpers (same [`super::ArcSweep`] payload; LOD matches linear).

/// Marker type for header-arc LOD docs; banding reuses [`crate::partitions::geometry::LinearLod`].
#[allow(dead_code)]
pub struct HeaderArcLod;
