//! 15° angular rough stonework partition for curved door/window framing.

use crate::assets::partitions::rough_stonework::ARC_15;
/// Narrow arc sweep used to compose circular openings.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonework15;


crate::impl_glb_lod_scene!(RoughStonework15, ARC_15);
