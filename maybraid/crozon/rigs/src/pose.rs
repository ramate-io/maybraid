//! Layered bind-pose composition for rig proportions.
//!
//! These types describe proportional effects as ordered scale / translation
//! multipliers and optional rotations on top of a captured bind pose. They
//! complement [`super::RigPose`], which stores absolute per-bone transforms for
//! animation and slider insertion; use this module when effects should compose
//! as `bind * layer1 * layer2 * …`.
//!
//! Prefer [`BoneTranslation::length`] over [`BoneScale::length`] when a bone
//! also needs a pose rotation and has descendants that must stay unsheared
//! (non-uniform scale + child rotation shears `GlobalTransform`).

use bevy::prelude::*;

/// Scale multiplier for one named bone.
#[derive(Debug, Clone, PartialEq)]
pub struct BoneScale {
	pub bone: &'static str,
	pub scale: Vec3,
}

impl BoneScale {
	pub const fn new(bone: &'static str, scale: Vec3) -> Self {
		Self { bone, scale }
	}

	pub fn uniform(bone: &'static str, scale: f32) -> Self {
		Self { bone, scale: Vec3::splat(scale) }
	}

	pub fn length(bone: &'static str, scale: f32) -> Self {
		// Humanoid proportion bones usually carry length along local Y.
		Self { bone, scale: Vec3::new(1.0, scale, 1.0) }
	}

	pub fn lateral(bone: &'static str, scale: f32) -> Self {
		// Width groups on this rig tend to use local X.
		Self { bone, scale: Vec3::new(scale, 1.0, 1.0) }
	}

	pub fn thickness(bone: &'static str, scale: f32) -> Self {
		// Bulk controls that should not stretch bone length.
		Self { bone, scale: Vec3::new(scale, 1.0, scale) }
	}
}

/// Bind-pose translation multiplier for one named bone.
///
/// Scales the captured bind translation component-wise. Use this to lengthen a
/// bone chain without introducing non-uniform scale (which shears rotated
/// descendants).
#[derive(Debug, Clone, PartialEq)]
pub struct BoneTranslation {
	pub bone: &'static str,
	pub multiplier: Vec3,
}

impl BoneTranslation {
	pub const fn new(bone: &'static str, multiplier: Vec3) -> Self {
		Self { bone, multiplier }
	}

	/// Stretch bind translation along local Y (typical bone length axis).
	pub fn length(bone: &'static str, scale: f32) -> Self {
		Self { bone, multiplier: Vec3::new(1.0, scale, 1.0) }
	}
}

/// Rotation offset for one named bone, composed onto the bind rotation.
///
/// Pose apply uses parent-space composition (`delta * bind`) so a counter-pitch
/// on a non-identity rest bone (e.g. `head_socket`) can cancel a parent pitch.
#[derive(Debug, Clone, PartialEq)]
pub struct BoneRotation {
	pub bone: &'static str,
	pub rotation: Quat,
}

impl BoneRotation {
	pub const fn new(bone: &'static str, rotation: Quat) -> Self {
		Self { bone, rotation }
	}

	pub fn pitch_x(bone: &'static str, radians: f32) -> Self {
		Self { bone, rotation: Quat::from_rotation_x(radians) }
	}

	/// Pitch about local Z — use when the armature rest orients length along Y
	/// after a 90° Z export flip (common for Blender Y-up necks).
	pub fn pitch_z(bone: &'static str, radians: f32) -> Self {
		Self { bone, rotation: Quat::from_rotation_z(radians) }
	}
}

/// A named proportional layer in the bind-pose composition stack.
#[derive(Debug, Clone, PartialEq)]
pub struct RigPoseLayer {
	pub label: &'static str,
	pub scales: Vec<BoneScale>,
	pub translations: Vec<BoneTranslation>,
	pub rotations: Vec<BoneRotation>,
}

impl RigPoseLayer {
	pub fn new(label: &'static str) -> Self {
		Self { label, scales: Vec::new(), translations: Vec::new(), rotations: Vec::new() }
	}

	pub fn with_scale(mut self, scale: BoneScale) -> Self {
		self.scales.push(scale);
		self
	}

	pub fn with_translation(mut self, translation: BoneTranslation) -> Self {
		self.translations.push(translation);
		self
	}

	pub fn with_rotation(mut self, rotation: BoneRotation) -> Self {
		self.rotations.push(rotation);
		self
	}

	pub fn scales(&self) -> impl Iterator<Item = &BoneScale> {
		self.scales.iter()
	}

	pub fn translations(&self) -> impl Iterator<Item = &BoneTranslation> {
		self.translations.iter()
	}

	pub fn rotations(&self) -> impl Iterator<Item = &BoneRotation> {
		self.rotations.iter()
	}
}

/// Resolved layers to apply to a rig, in order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedRigPose {
	pub layers: Vec<RigPoseLayer>,
}

impl ResolvedRigPose {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_layer(mut self, layer: RigPoseLayer) -> Self {
		self.layers.push(layer);
		self
	}

	pub fn layers(&self) -> impl Iterator<Item = &RigPoseLayer> {
		self.layers.iter()
	}

	pub fn scale_for_bone(&self, bone: &str) -> Vec3 {
		// Same bone may appear in multiple layers; multiply in stack order.
		self.layers
			.iter()
			.flat_map(|layer| layer.scales())
			.filter(|scale| scale.bone == bone)
			.fold(Vec3::ONE, |acc, scale| acc * scale.scale)
	}

	pub fn translation_for_bone(&self, bone: &str) -> Vec3 {
		self.layers
			.iter()
			.flat_map(|layer| layer.translations())
			.filter(|translation| translation.bone == bone)
			.fold(Vec3::ONE, |acc, translation| acc * translation.multiplier)
	}

	pub fn rotation_for_bone(&self, bone: &str) -> Quat {
		// Same bone may appear in multiple layers; compose in stack order.
		self.layers
			.iter()
			.flat_map(|layer| layer.rotations())
			.filter(|rotation| rotation.bone == bone)
			.fold(Quat::IDENTITY, |acc, rotation| acc * rotation.rotation)
	}
}
