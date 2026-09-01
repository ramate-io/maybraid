//! Playground / recipe proportion pose for kit bones.

use firearms_components::{BoneScale, ResolvedRigPose, RigPoseLayer};

use crate::parts::KitBone;

/// Length (bone local Y) and thickness (bone local XZ) multipliers. `1.0` is bind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneFit {
	pub length: f32,
	pub thickness: f32,
}

impl Default for BoneFit {
	fn default() -> Self {
		Self { length: 1.0, thickness: 1.0 }
	}
}

/// Per-kit-bone length / thickness. Applied through [`ActiveRigPose`](firearms_components::ActiveRigPose).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FirearmPose {
	pub body: BoneFit,
	pub barrel: BoneFit,
	pub trigger_box: BoneFit,
	pub grip: BoneFit,
	pub stock: BoneFit,
}

impl FirearmPose {
	pub fn fit(self, bone: KitBone) -> BoneFit {
		match bone {
			KitBone::Body => self.body,
			KitBone::Barrel => self.barrel,
			KitBone::TriggerBox => self.trigger_box,
			KitBone::Grip => self.grip,
			KitBone::Stock => self.stock,
		}
	}

	pub fn fit_mut(&mut self, bone: KitBone) -> &mut BoneFit {
		match bone {
			KitBone::Body => &mut self.body,
			KitBone::Barrel => &mut self.barrel,
			KitBone::TriggerBox => &mut self.trigger_box,
			KitBone::Grip => &mut self.grip,
			KitBone::Stock => &mut self.stock,
		}
	}

	pub fn to_resolved(self) -> ResolvedRigPose {
		let mut layer = RigPoseLayer::new("kit");
		for bone in KitBone::VALUES {
			let fit = self.fit(*bone);
			let name = bone.bone_name();
			layer = layer
				.with_scale(BoneScale::length(name, fit.length))
				.with_scale(BoneScale::thickness(name, fit.thickness));
		}
		ResolvedRigPose::new().with_layer(layer)
	}

	pub fn label(self) -> String {
		let mut parts = Vec::new();
		for bone in KitBone::VALUES {
			let fit = self.fit(*bone);
			if (fit.length - 1.0).abs() > f32::EPSILON || (fit.thickness - 1.0).abs() > f32::EPSILON
			{
				parts.push(format!("{} L={} T={}", bone.label(), fit.length, fit.thickness));
			}
		}
		if parts.is_empty() {
			"bind".into()
		} else {
			parts.join(" ")
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::Vec3;

	#[test]
	fn length_and_thickness_compose_on_y_and_xz() {
		let mut pose = FirearmPose::default();
		pose.barrel.length = 2.0;
		pose.barrel.thickness = 0.5;
		assert_eq!(pose.to_resolved().scale_for_bone("barrel"), Vec3::new(0.5, 2.0, 0.5));
		assert_eq!(pose.to_resolved().scale_for_bone("body"), Vec3::ONE);
	}
}
