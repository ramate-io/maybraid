//! Braidman proportion layers.
//!
//! This module documents the first slice of Braidman's species silhouette. It
//! does not overwrite bind-pose transforms; it produces named bone scale layers
//! that the preview applies to the loaded rest pose.

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::braidman::{sliders::BraidmanSliders, BraidmanConfig},
};
use crozon_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

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
		// Order matches spec Stage 1: baseline → gender → build → user sliders.
		// Each layer multiplies; later layers never replace earlier bone scales.
		ResolvedRigPose::new()
			.with_layer(self.species_baseline())
			.with_layer(self.gender_layer())
			.with_layer(self.build_layer())
			.with_layer(self.slider_layer())
	}

	fn species_baseline(self) -> RigPoseLayer {
		// Authored humanoid rest pose is not Braidman's silhouette; these constants
		// are the species-owned baseline before presets or command sliders run.
		RigPoseLayer::new("braidman species baseline")
			.with_scale(BoneScale::uniform("chest.L", 0.8))
			.with_scale(BoneScale::uniform("chest.R", 0.8))
			.with_scale(BoneScale::uniform("lat.L", 0.2))
			.with_scale(BoneScale::uniform("lat.R", 0.2))
			.with_scale(BoneScale::uniform("waist.L", 1.0))
			.with_scale(BoneScale::uniform("waist.R", 1.0))
			.with_scale(BoneScale::length("pelvis.L", 0.8))
			.with_scale(BoneScale::length("pelvis.R", 0.8))
			.with_scale(BoneScale::uniform("lumbar", 0.8))
			.with_scale(BoneScale::uniform("buttocks", 0.8))
			.with_scale(BoneScale::thickness("humerus.L", 0.8))
			.with_scale(BoneScale::thickness("humerus.R", 0.8))
			.with_scale(BoneScale::uniform("thigh_thickness.L", 0.5))
			.with_scale(BoneScale::uniform("thigh_thickness.R", 0.5))
	}

	fn gender_layer(self) -> RigPoseLayer {
		// Lean shortcut: preset effects are bone scales here. Full pass should
		// apply spec percent offsets to `BraidmanSliders` in `presets` first.
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = Self::with_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = Self::with_shoulder_width(layer, 0.9);
				layer = Self::with_chest_thickness(layer, 1.5);
				layer = Self::with_hip_width(layer, 1.2);
				layer = Self::with_hip_thickness(layer, 1.1);
				layer = Self::with_thigh_thickness(layer, 1.2);
				layer = Self::with_buttocks_thickness(layer, 1.2);
				layer = Self::with_lower_trunk_thickness(layer, 0.9);
				layer = Self::with_waist_thickness(layer, 0.7);
				// Narrower shoulders shorten reach unless arm bones lengthen too.
				layer = Self::with_arm_length(layer, 1.1);
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
		// Command/UI overrides win last; same bone helpers as presets for now.
		RigPoseLayer::new("command sliders")
			.with_scale(BoneScale::uniform("shoulder.L", self.sliders.shoulder_width))
			.with_scale(BoneScale::uniform("shoulder.R", self.sliders.shoulder_width))
			.with_scale(BoneScale::uniform("pelvis.L", self.sliders.hip_width))
			.with_scale(BoneScale::uniform("pelvis.R", self.sliders.hip_width))
			.with_scale(BoneScale::thickness("chest_thickness", self.sliders.chest_thickness))
	}

	fn with_shoulder_width(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		// Shoulder bones carry width in uniform local scale on this rig.
		layer
			.with_scale(BoneScale::uniform("shoulder.L", value))
			.with_scale(BoneScale::uniform("shoulder.R", value))
	}

	fn with_hip_width(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		// Pelvis width is authored along bone Y (length), not lateral X.
		layer
			.with_scale(BoneScale::length("pelvis.L", value))
			.with_scale(BoneScale::length("pelvis.R", value))
	}

	fn with_hip_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::thickness("pelvis.L", value))
			.with_scale(BoneScale::thickness("pelvis.R", value))
	}

	fn with_chest_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		// Ventrally oriented control bone: uniform scale, not thickness helper.
		layer.with_scale(BoneScale::uniform("chest_thickness", value))
	}

	fn with_thigh_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::uniform("thigh_thickness.L", value))
			.with_scale(BoneScale::uniform("thigh_thickness.R", value))
	}

	fn with_buttocks_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer.with_scale(BoneScale::uniform("buttocks", value))
	}

	fn with_waist_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("waist.L", value))
			.with_scale(BoneScale::length("waist.R", value))
	}

	fn with_lower_trunk_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer.with_scale(BoneScale::thickness("lumbar", value))
	}

	fn with_arm_length(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("humerus.L", value))
			.with_scale(BoneScale::length("humerus.R", value))
			.with_scale(BoneScale::length("forearm.L", value))
			.with_scale(BoneScale::length("forearm.R", value))
	}
}
