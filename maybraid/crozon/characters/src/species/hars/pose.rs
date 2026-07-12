//! Hars proportion layers on the quadruped rig.
//!
//! Horse-like carriage: rumbler torso baseline with a lengthened pitched neck
//! and taller limbs. Head leveling is a counter-pitch on the `head_socket`
//! bone — the head rig sockets in with an identity transform.
//!
//! Length is a bind-translation stretch on `head_socket` (along neck-local Y),
//! not non-uniform scale on `neck`. Scaling `neck` on Y while rotating
//! `head_socket` shears the head; stretching `neck`'s own translation extends
//! in parent space (along the spine) and flattens the raised silhouette.

use std::f32::consts::FRAC_PI_4;

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::hars::{sliders::HarsSliders, HarsConfig},
};
use crozon_rigs::{
	BoneRotation, BoneScale, BoneTranslation, ResolvedRigPose, RigPoseLayer,
};

/// Raise the neck bone.
pub const NECK_PITCH: f32 = FRAC_PI_4;
/// Counter-pitch on `head_socket`. Equal magnitude opposite sign to
/// [`NECK_PITCH`] restores the head's pre-neck-pitch alignment.
pub const HEAD_SOCKET_PITCH: f32 = -NECK_PITCH;

/// Push `head_socket` out along neck-local Y so the pitched neck reads long.
const NECK_LENGTH: f32 = 4.5;
/// Bulk the neck mesh on XZ (length stays on `head_socket` translation).
const NECK_THICKNESS: f32 = 2.0;
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

	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
			.with_layer(self.species_baseline())
			.with_layer(
				RigPoseLayer::new("hars neck pitch")
					.with_rotation(BoneRotation::pitch_x("neck", NECK_PITCH)),
			)
			.with_layer(
				RigPoseLayer::new("hars head socket pitch")
					.with_rotation(BoneRotation::pitch_x("head_socket", HEAD_SOCKET_PITCH)),
			)
			.with_layer(self.gender_layer())
			.with_layer(self.build_layer())
			.with_layer(self.slider_layer())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("hars species baseline")
			.with_translation(BoneTranslation::length("head_socket", NECK_LENGTH))
			.with_scale(BoneScale::thickness("neck", NECK_THICKNESS))
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
