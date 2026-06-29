//! Braidman proportion layers.
//!
//! This module documents the first slice of Braidman's species silhouette. It
//! does not overwrite bind-pose transforms; it produces named bone scale layers
//! that the preview applies to the loaded rest pose.

use crate::{
	pose::{BoneScale, ResolvedRigPose, RigPoseLayer},
	presets::{BuildPreset, GenderPreset},
	species::braidman::{sliders::BraidmanSliders, BraidmanConfig},
};

/// Resolved proportional intent for Braidman's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BraidmanPose {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: BraidmanSliders,
}

impl BraidmanPose {
	pub fn from_config(config: &BraidmanConfig) -> Self {
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
		RigPoseLayer::new("braidman species baseline")
			.with_scale(BoneScale::lateral("chest.L", 0.8))
			.with_scale(BoneScale::lateral("chest.R", 0.8))
			.with_scale(BoneScale::lateral("waist.L", 1.2))
			.with_scale(BoneScale::lateral("waist.R", 1.2))
	}

	fn gender_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = Self::with_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = Self::with_shoulder_width(layer, 0.95);
				layer = Self::with_chest_thickness(layer, 1.2);
				layer = Self::with_hip_width(layer, 1.1);
			}
			GenderPreset::NonBinary => {
				layer = Self::with_shoulder_width(layer, 0.95);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = Self::with_shoulder_width(layer, 0.95);
				layer = Self::with_chest_thickness(layer, 0.9);
				layer = Self::with_hip_width(layer, 0.9);
			}
			BuildPreset::Athletic => {
				layer = Self::with_shoulder_width(layer, 1.05);
				layer = Self::with_chest_thickness(layer, 1.1);
				layer = Self::with_hip_width(layer, 1.1);
			}
			BuildPreset::Heavy => {
				layer = Self::with_shoulder_width(layer, 1.05);
				layer = Self::with_chest_thickness(layer, 1.1);
				layer = Self::with_hip_width(layer, 1.1);
			}
			BuildPreset::Stocky => {
				layer = Self::with_shoulder_width(layer, 1.1);
				layer = Self::with_chest_thickness(layer, 1.1);
			}
			BuildPreset::Lanky => {
				layer = Self::with_shoulder_width(layer, 0.95);
				layer = Self::with_chest_thickness(layer, 0.9);
				layer = Self::with_hip_width(layer, 0.9);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		RigPoseLayer::new("command sliders")
			.with_scale(BoneScale::uniform("shoulder.L", self.sliders.shoulder_width))
			.with_scale(BoneScale::uniform("shoulder.R", self.sliders.shoulder_width))
			.with_scale(BoneScale::uniform("pelvis.L", self.sliders.hip_width))
			.with_scale(BoneScale::uniform("pelvis.R", self.sliders.hip_width))
			.with_scale(BoneScale::thickness("chest_thickness", self.sliders.chest_thickness))
	}

	fn with_shoulder_width(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::uniform("shoulder.L", value))
			.with_scale(BoneScale::uniform("shoulder.R", value))
	}

	fn with_hip_width(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::uniform("pelvis.L", value))
			.with_scale(BoneScale::uniform("pelvis.R", value))
	}

	fn with_chest_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer.with_scale(BoneScale::thickness("chest_thickness", value))
	}
}
