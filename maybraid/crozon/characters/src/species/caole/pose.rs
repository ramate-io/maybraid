//! Caole proportion layers on the quadruped rig.

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::caole::{sliders::CaoleSliders, CaoleConfig},
};
use crozon_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Caole's quadruped rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaolePose {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: CaoleSliders,
}

impl CaolePose {
	pub fn from_config(config: &CaoleConfig) -> Self {
		Self { gender: config.gender, build: config.build, sliders: config.sliders.clamped() }
	}

	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
			.with_layer(self.species_baseline())
			.with_layer(self.gender_layer())
			.with_layer(self.build_layer())
			.with_layer(self.slider_layer())
	}

	fn species_baseline(self) -> RigPoseLayer {
		RigPoseLayer::new("caole species baseline")
			.with_scale(BoneScale::uniform("chest_thickness", 1.0))
			.with_scale(BoneScale::uniform("belly", 1.0))
	}

	fn gender_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = CaoleSliders::apply_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = CaoleSliders::apply_shoulder_width(layer, 0.95);
				layer = CaoleSliders::apply_chest_thickness(layer, 1.1);
				layer = CaoleSliders::apply_hip_width(layer, 1.1);
				layer = CaoleSliders::apply_hip_thickness(layer, 1.05);
				layer = CaoleSliders::apply_buttocks_thickness(layer, 1.1);
			}
			GenderPreset::NonBinary => {
				layer = CaoleSliders::apply_shoulder_width(layer, 0.98);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = CaoleSliders::apply_chest_thickness(layer, 0.95);
				layer = CaoleSliders::apply_hip_width(layer, 0.95);
			}
			BuildPreset::Athletic | BuildPreset::Heavy => {
				layer = CaoleSliders::apply_chest_thickness(layer, 1.05);
				layer = CaoleSliders::apply_hip_width(layer, 1.05);
			}
			BuildPreset::Stocky => {
				layer = CaoleSliders::apply_shoulder_width(layer, 1.05);
				layer = CaoleSliders::apply_chest_thickness(layer, 1.05);
			}
			BuildPreset::Lanky => {
				layer = CaoleSliders::apply_leg_length(layer, 1.05);
				layer = CaoleSliders::apply_arm_length(layer, 1.05);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		self.sliders.apply_slider_layer()
	}
}
