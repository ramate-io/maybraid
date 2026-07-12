//! Hars proportion layers on the quadruped body + triple-join neck.
//!
//! Carriage comes from pitching the separate neck armature (uniform path, no
//! non-uniform body-neck scale). The head sockets onto the neck tip with a
//! matching counter-pitch on the neck's `head_socket` bone.

use std::f32::consts::FRAC_PI_4;

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::hars::{sliders::HarsSliders, HarsConfig},
};
use crozon_rigs::{BoneRotation, BoneScale, ResolvedRigPose, RigPoseLayer};

/// Total raise across the triple-join neck; distributed evenly per segment.
pub const NECK_PITCH: f32 = FRAC_PI_4;
/// Counter-pitch on the neck tip `head_socket` so the head stays level.
pub const HEAD_SOCKET_PITCH: f32 = -NECK_PITCH;

const SEGMENT_PITCH: f32 = NECK_PITCH / 3.0;

/// Elevated limb length relative to the stock quadruped / caole baselines.
const LIMB_LENGTH: f32 = 1.35;
/// Rumbler torso mass (same bones as caole rumbler).
const TORSO_THICKNESS: f32 = 1.35;
const RUMBLER_BACK_RIDGE_LENGTH: f32 = 1.45;
const RUMBLER_LEG_THICKNESS: f32 = 1.15;
const RUMBLER_BELLY_LENGTH: f32 = 2.0;

/// Resolved proportional intent for Hars's horse-like quadruped rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarsPose {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: HarsSliders,
}

impl HarsPose {
	pub fn from_config(config: &HarsConfig) -> Self {
		Self { gender: config.gender, build: config.build, sliders: config.sliders.clamped() }
	}

	/// Body-rig layers (torso / limbs). Neck pitch lives on [`Self::neck_pose`].
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
			.with_layer(self.species_baseline())
			.with_layer(self.gender_layer())
			.with_layer(self.build_layer())
			.with_layer(self.slider_layer())
	}

	/// Triple-join neck OwnRig layers: distribute pitch, counter-pitch tip.
	pub fn neck_pose(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(
			RigPoseLayer::new("hars neck pitch")
				.with_rotation(BoneRotation::pitch_x("neck_base", SEGMENT_PITCH))
				.with_rotation(BoneRotation::pitch_x("mid_neck", SEGMENT_PITCH))
				.with_rotation(BoneRotation::pitch_x("upper_neck", SEGMENT_PITCH))
				.with_rotation(BoneRotation::pitch_x("head_socket", HEAD_SOCKET_PITCH)),
		)
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("hars species baseline")
			.with_scale(BoneScale::length("back_ridge", RUMBLER_BACK_RIDGE_LENGTH))
			.with_scale(BoneScale::thickness("anterior_mid_back", TORSO_THICKNESS))
			.with_scale(BoneScale::thickness("posterior_mid_back", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("waist.L", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("waist.R", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("lower_chest_width.L", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("lower_chest_width.R", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("chest_thickness", 1.0))
			.with_scale(BoneScale::length("belly", RUMBLER_BELLY_LENGTH))
			.with_scale(BoneScale::uniform("belly", 1.0));
		layer = HarsSliders::apply_leg_thickness(layer, RUMBLER_LEG_THICKNESS);
		layer = HarsSliders::apply_arm_length(layer, LIMB_LENGTH);
		HarsSliders::apply_leg_length(layer, LIMB_LENGTH)
	}

	fn gender_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = HarsSliders::apply_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = HarsSliders::apply_shoulder_width(layer, 0.95);
				layer = HarsSliders::apply_chest_thickness(layer, 1.1);
				layer = HarsSliders::apply_hip_width(layer, 1.1);
				layer = HarsSliders::apply_hip_thickness(layer, 1.05);
				layer = HarsSliders::apply_buttocks_thickness(layer, 1.1);
			}
			GenderPreset::NonBinary => {
				layer = HarsSliders::apply_shoulder_width(layer, 0.98);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = HarsSliders::apply_chest_thickness(layer, 0.95);
				layer = HarsSliders::apply_hip_width(layer, 0.95);
			}
			BuildPreset::Athletic | BuildPreset::Heavy => {
				layer = HarsSliders::apply_chest_thickness(layer, 1.05);
				layer = HarsSliders::apply_hip_width(layer, 1.05);
			}
			BuildPreset::Stocky => {
				layer = HarsSliders::apply_shoulder_width(layer, 1.05);
				layer = HarsSliders::apply_chest_thickness(layer, 1.05);
			}
			BuildPreset::Lanky => {
				layer = HarsSliders::apply_leg_length(layer, 1.05);
				layer = HarsSliders::apply_arm_length(layer, 1.05);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		self.sliders.apply_slider_layer()
	}
}
