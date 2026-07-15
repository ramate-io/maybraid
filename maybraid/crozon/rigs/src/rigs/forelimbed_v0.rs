use bevy::prelude::*;

use crate::{
	forelimbed::{ForelimbedFin, ForelimbedRig, ForelimbedSpine},
	BoneDefinition, BonePose, BoneTable, Name, RigPose, RiggedAxis, Side,
};

/// Imported forelimbed (aquatic) rig: axial spine + paired pectoral fins.
#[derive(Component, Debug, Clone)]
pub struct ForelimbedV0Rig {
	pub bones: BoneTable,
	pub pose: RigPose,
}

impl ForelimbedV0Rig {
	pub fn imported() -> Self {
		let mut bones = BoneTable::new();
		for (name, relative_axis) in FORELIMBED_V0_BONE_DEFINITIONS {
			bones.insert(BoneDefinition { name: Name::from(name), relative_axis });
		}

		Self { bones, pose: RigPose::new() }
	}

	fn bone_pose(&self, name: impl Into<Name>) -> BonePose {
		let name = name.into();
		self.pose
			.get(&name)
			.cloned()
			.unwrap_or_else(|| BonePose::new(name, Transform::IDENTITY))
	}

	fn local_rotation(&self, bone: &Name) -> Quat {
		self.pose
			.get(bone)
			.map(|pose| pose.transform.rotation)
			.unwrap_or(Quat::IDENTITY)
	}

	fn world_rotation_for(&self, bone: &Name) -> Quat {
		self.parent_world_rotation_for(bone) * self.local_rotation(bone)
	}

	fn parent_world_rotation_for(&self, bone: &Name) -> Quat {
		forelimbed_v0_parent(bone.as_str())
			.map(|parent| self.world_rotation_for(&Name::from(parent)))
			.unwrap_or(Quat::IDENTITY)
	}

	pub fn animation_bones(&self) -> Vec<Name> {
		let spine = self.spine();
		let left = self.fin(Side::Left);
		let right = self.fin(Side::Right);
		vec![
			spine.upper_mid_spine.name,
			spine.upper_spine.name,
			spine.lower_mid_spine.name,
			spine.lower_spine.name,
			spine.tailbone.name,
			spine.back_ridge.name,
			left.shoulder.name,
			left.upper_arm.name,
			left.lower_arm.name,
			right.shoulder.name,
			right.upper_arm.name,
			right.lower_arm.name,
		]
	}
}

impl ForelimbedRig for ForelimbedV0Rig {
	fn spine(&self) -> ForelimbedSpine {
		ForelimbedSpine {
			upper_mid_spine: self.bone_pose("upper_mid_spine"),
			upper_spine: self.bone_pose("upper_spine"),
			lower_mid_spine: self.bone_pose("lower_mid_spine"),
			lower_spine: self.bone_pose("lower_spine"),
			tailbone: self.bone_pose("tailbone"),
			back_ridge: self.bone_pose("back_ridge"),
		}
	}

	fn fin(&self, side: Side) -> ForelimbedFin {
		let suffix = side.suffix();
		ForelimbedFin {
			shoulder: self.bone_pose(format!("shoulder.{suffix}")),
			upper_arm: self.bone_pose(format!("upper_arm.{suffix}")),
			lower_arm: self.bone_pose(format!("lower_arm.{suffix}")),
		}
	}

	fn pose(&self) -> &RigPose {
		&self.pose
	}

	fn pose_mut(&mut self) -> &mut RigPose {
		&mut self.pose
	}

	fn rigged_axis(&self, bone: &Name) -> Option<RiggedAxis> {
		self.bones.get(bone).map(|bone| bone.relative_axis)
	}

	fn animation_bones(&self) -> Vec<Name> {
		ForelimbedV0Rig::animation_bones(self)
	}

	fn parent_world_rotation(&self, bone: &Name) -> Quat {
		self.parent_world_rotation_for(bone)
	}
}

impl Default for ForelimbedV0Rig {
	fn default() -> Self {
		Self::imported()
	}
}

pub const FORELIMBED_V0_BONE_DEFINITIONS: [(&str, RiggedAxis); 18] = [
	("lower_mid_spine", RiggedAxis::DEFAULT),
	("lower_spine", RiggedAxis::DEFAULT),
	("tailbone", RiggedAxis::DEFAULT),
	("tail_socket", RiggedAxis::DEFAULT),
	("upper_mid_spine", RiggedAxis::DEFAULT),
	("upper_spine", RiggedAxis::DEFAULT),
	("head_socket", RiggedAxis::DEFAULT),
	("shoulder.L", RiggedAxis::DEFAULT),
	("upper_arm.L", RiggedAxis::DEFAULT),
	("lower_arm.L", RiggedAxis::DEFAULT),
	("shoulder.R", RiggedAxis::DEFAULT),
	("upper_arm.R", RiggedAxis::DEFAULT),
	("lower_arm.R", RiggedAxis::DEFAULT),
	("torso_thickness.L", RiggedAxis::DEFAULT),
	("belly", RiggedAxis::DEFAULT),
	("back_ridge", RiggedAxis::DEFAULT),
	("dorsal_socket", RiggedAxis::DEFAULT),
	("torso_thickness.R", RiggedAxis::DEFAULT),
];

pub fn forelimbed_v0_bone_names() -> impl Iterator<Item = &'static str> {
	FORELIMBED_V0_BONE_DEFINITIONS.into_iter().map(|(name, _axis)| name)
}

const FORELIMBED_V0_PARENT: &[(&str, &str)] = &[
	("lower_mid_spine", ""),
	("lower_spine", "lower_mid_spine"),
	("tailbone", "lower_spine"),
	("tail_socket", "tailbone"),
	("upper_mid_spine", ""),
	("upper_spine", "upper_mid_spine"),
	("head_socket", "upper_spine"),
	("shoulder.L", "upper_spine"),
	("upper_arm.L", "shoulder.L"),
	("lower_arm.L", "upper_arm.L"),
	("shoulder.R", "upper_spine"),
	("upper_arm.R", "shoulder.R"),
	("lower_arm.R", "upper_arm.R"),
	("back_ridge", ""),
	("dorsal_socket", "back_ridge"),
];

fn forelimbed_v0_parent(name: &str) -> Option<&'static str> {
	FORELIMBED_V0_PARENT
		.iter()
		.find(|(child, _)| *child == name)
		.map(|(_, parent)| *parent)
		.filter(|parent| !parent.is_empty())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn forelimbed_v0_accessors_map_to_imported_names() {
		let rig = ForelimbedV0Rig::imported();
		assert_eq!(rig.spine().tailbone.name, Name::from("tailbone"));
		assert_eq!(rig.fin(Side::Left).upper_arm.name, Name::from("upper_arm.L"));
		assert_eq!(rig.fin(Side::Right).lower_arm.name, Name::from("lower_arm.R"));
	}

	#[test]
	fn forelimbed_v0_animation_bones_exist_in_definition_table() {
		let rig = ForelimbedV0Rig::imported();
		for name in ForelimbedRig::animation_bones(&rig) {
			assert!(rig.bones.get(&name).is_some(), "missing animation bone {name}");
		}
	}

	#[test]
	fn forelimbed_v0_definition_covers_imported_dump() {
		let rig = ForelimbedV0Rig::imported();
		for name in forelimbed_v0_bone_names() {
			assert!(rig.bones.get(&Name::from(name)).is_some(), "missing bone {name}");
		}
		assert_eq!(rig.bones.len(), FORELIMBED_V0_BONE_DEFINITIONS.len());
	}
}
