//! Spibmom proportion baseline.
//!
//! Wumbus body silhouette with a longer neck for the enlarged meerkat head.

use crate::species::{
	braidman::sliders::BraidmanSliders,
	wumbus::pose::WumbusPose,
};
use crozon_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Spibmom's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpibmomPose;

impl SpibmomPose {
	pub fn resolve(self) -> ResolvedRigPose {
		WumbusPose.resolve().with_layer(self.neck_layer())
	}

	fn neck_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("spibmom neck");

		layer = layer
			.with_scale(BoneScale::length("lower_neck", 1.3))
			.with_scale(BoneScale::length("upper_neck", 1.3));

		layer = BraidmanSliders::apply_arm_length(layer, 1.05);

		layer
	}
}
