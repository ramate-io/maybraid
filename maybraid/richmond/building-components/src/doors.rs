//! Door frame and leaf scene components.
//!
//! A common rough-stone door frame uses a header with 15° arc sweeps
//! (see crate README). Leaves may be wood.

use bevy::scene::prelude::{bsn, Scene};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::partitions::rough_stonework::{
	RoughStonework15, RoughStoneworkHeader15, RoughStoneworkLinearHeaderSubsegment,
};

/// Rough stone door frame composed from header + 15° arc kit pieces.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneDoorFrame15 {
	pub header_left: RoughStoneworkHeader15,
	pub header_right: RoughStoneworkHeader15,
	pub header_span: RoughStoneworkLinearHeaderSubsegment,
	pub jamb_left: RoughStonework15,
	pub jamb_right: RoughStonework15,
}

impl LodScene for RoughStoneDoorFrame15 {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(self.header_left.scene_with_lod(lod_ref)),
			Box::new(self.header_right.scene_with_lod(lod_ref)),
			Box::new(self.header_span.scene_with_lod(lod_ref)),
			Box::new(self.jamb_left.scene_with_lod(lod_ref)),
			Box::new(self.jamb_right.scene_with_lod(lod_ref)),
		];
		bsn! {
			Children [ {children} ]
		}
	}
}

/// Wood door leaf hung in a stone (or wood) frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodDoorLeaf;

crate::impl_empty_lod_scene!(WoodDoorLeaf);
