//! Rig-pose effect data for character proportions.
//!
//! The first concepts playground pass keeps pose data intentionally small. Each
//! layer describes scale multipliers that should compose with the loaded bind
//! pose. A preview system can apply these to live Bevy bone transforms after the
//! rig's bone map is available.

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

	pub fn lateral(bone: &'static str, scale: f32) -> Self {
		Self { bone, scale: Vec3::new(scale, 1.0, 1.0) }
	}

	pub fn thickness(bone: &'static str, scale: f32) -> Self {
		Self { bone, scale: Vec3::new(scale, 1.0, scale) }
	}
}

/// A named proportional layer in the bind-pose composition stack.
#[derive(Debug, Clone, PartialEq)]
pub struct RigPoseLayer {
	pub label: &'static str,
	pub scales: Vec<BoneScale>,
}

impl RigPoseLayer {
	pub fn new(label: &'static str) -> Self {
		Self { label, scales: Vec::new() }
	}

	pub fn with_scale(mut self, scale: BoneScale) -> Self {
		self.scales.push(scale);
		self
	}

	pub fn scales(&self) -> impl Iterator<Item = &BoneScale> {
		self.scales.iter()
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
		self.layers
			.iter()
			.flat_map(|layer| layer.scales())
			.filter(|scale| scale.bone == bone)
			.fold(Vec3::ONE, |acc, scale| acc * scale.scale)
	}
}
