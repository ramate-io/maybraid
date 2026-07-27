//! Rough stonework partition variants used by circular towers and door frames.
//!
//! Each type maps from a [`crate::partitions::geometry_components::WallComponent`] kit piece.

pub mod arc_15;
pub mod arc_180;
pub mod arc_90;
pub mod header_15;
pub mod header_180;
pub mod header_90;
pub mod linear;
pub mod linear_header_subsegment;
pub mod linear_subsegment;

pub use arc_15::RoughStonework15;
pub use arc_180::RoughStonework180;
pub use arc_90::RoughStonework90;
pub use header_15::RoughStoneworkHeader15;
pub use header_180::RoughStoneworkHeader180;
pub use header_90::RoughStoneworkHeader90;
pub use linear::RoughStoneworkLinear;
pub use linear_header_subsegment::RoughStoneworkLinearHeaderSubsegment;
pub use linear_subsegment::RoughStoneworkLinearSubsegment;
