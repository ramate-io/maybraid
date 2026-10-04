use bevy::prelude::*;

use crate::{
	authoring::{
		quadruped_v0_definition, resolve_quadruped, PoseBuffer, PoseScratch, QuadrupedPose,
		RigBinding, QUADRUPED_V0_BONES,
	},
	quadruped::LegSegmentLengths,
	BoneDefinition, BoneTable, Name, RiggedAxis,
};

/// Left thigh: sagittal stride on Y, medial/lateral on X, knee hinge lives on shin.
const QUADRUPED_V0_THIGH_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::X, twist_axis: Vec3::Z };

const QUADRUPED_V0_SHIN_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::Z, twist_axis: Vec3::X };

const QUADRUPED_V0_RIGHT_THIGH_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::NEG_Y, flex_axis: Vec3::NEG_X, twist_axis: Vec3::Z };

const QUADRUPED_V0_RIGHT_SHIN_AXIS: RiggedAxis =
	RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::NEG_Z, twist_axis: Vec3::X };

/// Store the bones of the imported quadruped rig in a semantically reasonable hierarchy.
#[derive(Component, Debug, Clone)]
pub struct QuadrupedV0Rig {
	pub bones: BoneTable,
	pub binding: RigBinding,
	/// Last resolved local pose. Sampling always starts from [`Self::binding`] rest.
	pub pose: PoseBuffer,
	pub scratch: PoseScratch,
	pub segment_lengths: LegSegmentLengths,
}

impl QuadrupedV0Rig {
	pub fn imported() -> Self {
		let mut bones = BoneTable::new();
		for (name, relative_axis) in QUADRUPED_V0_BONE_DEFINITIONS {
			bones.insert(BoneDefinition { name: Name::from(name), relative_axis });
		}
		let definition = quadruped_v0_definition();
		let len = definition.len();
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; len].into_boxed_slice(),
			PoseBuffer::identity(len),
		);
		Self {
			bones,
			pose: PoseBuffer::identity(len),
			scratch: PoseScratch::identity(len),
			binding,
			segment_lengths: LegSegmentLengths::default(),
		}
	}

	pub fn write_pose(&mut self, pose: &QuadrupedPose) {
		resolve_quadruped(pose, &self.binding, &mut self.pose);
		self.segment_lengths = self.binding.metrics.quadruped_leg;
	}

	pub fn rotation(&self, name: &str) -> Quat {
		self.binding
			.definition
			.id(name)
			.map(|id| self.pose.rotation(id))
			.unwrap_or(Quat::IDENTITY)
	}

	pub fn animation_bone_names(&self) -> impl Iterator<Item = &'static str> {
		QUADRUPED_V0_BONES.iter().copied()
	}

	pub fn rigged_axis(&self, bone: &Name) -> Option<RiggedAxis> {
		self.bones.get(bone).map(|bone| bone.relative_axis)
	}

	pub fn animation_bones(&self) -> Vec<Name> {
		self.animation_bone_names().map(Name::from).collect()
	}
}

impl Default for QuadrupedV0Rig {
	fn default() -> Self {
		Self::imported()
	}
}

pub const QUADRUPED_V0_BONE_DEFINITIONS: [(&str, RiggedAxis); 24] = [
	("back_ridge", RiggedAxis::DEFAULT),
	("upper_back", RiggedAxis::DEFAULT),
	("lumbar", RiggedAxis::DEFAULT),
	("neck", RiggedAxis::DEFAULT),
	("shoulder.L", RiggedAxis::DEFAULT),
	("anterior_thigh.L", QUADRUPED_V0_THIGH_AXIS),
	("anterior_shin.L", QUADRUPED_V0_SHIN_AXIS),
	("shoulder.R", RiggedAxis::DEFAULT),
	("anterior_thigh.R", QUADRUPED_V0_RIGHT_THIGH_AXIS),
	("anterior_shin.R", QUADRUPED_V0_RIGHT_SHIN_AXIS),
	("hip.L", RiggedAxis::DEFAULT),
	("posterior_thigh.L", QUADRUPED_V0_THIGH_AXIS),
	("posterior_shin.L", QUADRUPED_V0_SHIN_AXIS),
	("hip.R", RiggedAxis::DEFAULT),
	("posterior_thigh.R", QUADRUPED_V0_RIGHT_THIGH_AXIS),
	("posterior_shin.R", QUADRUPED_V0_RIGHT_SHIN_AXIS),
	("tailbone", RiggedAxis::DEFAULT),
	("head_socket", RiggedAxis::DEFAULT),
	("chest_thickness", RiggedAxis::DEFAULT),
	("belly", RiggedAxis::DEFAULT),
	("waist.L", RiggedAxis::DEFAULT),
	("waist.R", RiggedAxis::DEFAULT),
	("shoulder_vertical_thickness", RiggedAxis::DEFAULT),
	("haunch_vertical_thickness", RiggedAxis::DEFAULT),
];

pub fn quadruped_v0_bone_names() -> impl Iterator<Item = &'static str> {
	QUADRUPED_V0_BONE_DEFINITIONS.into_iter().map(|(name, _axis)| name)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Side;

	#[test]
	fn quadruped_v0_animation_bones_exist_in_definition_table() {
		let rig = QuadrupedV0Rig::imported();
		for name in rig.animation_bones() {
			assert!(rig.bones.get(&name).is_some(), "missing animation bone {name}");
			assert!(rig.binding.definition.id(name.as_str()).is_some(), "missing id {name}");
		}
	}

	#[test]
	fn quadruped_v0_definition_covers_imported_dump() {
		let rig = QuadrupedV0Rig::imported();
		for name in quadruped_v0_bone_names() {
			assert!(rig.bones.get(&Name::from(name)).is_some(), "missing bone {name}");
		}
		assert_eq!(rig.bones.len(), QUADRUPED_V0_BONE_DEFINITIONS.len());
	}

	#[test]
	fn same_positive_stride_and_hinge_bend_both_sides_sagittally() {
		let mut rig = QuadrupedV0Rig::imported();
		let mut pose = QuadrupedPose::default();
		for side in [Side::Left, Side::Right] {
			let front = pose.front_mut(side);
			front.stride = 0.5;
			front.hinge = 0.7;
			let hind = pose.hind_mut(side);
			hind.stride = 0.5;
			hind.hinge = 0.7;
		}
		rig.write_pose(&pose);

		for (left_name, right_name) in [
			("anterior_thigh.L", "anterior_thigh.R"),
			("anterior_shin.L", "anterior_shin.R"),
			("posterior_thigh.L", "posterior_thigh.R"),
			("posterior_shin.L", "posterior_shin.R"),
		] {
			let left = rig.rotation(left_name) * Vec3::Y;
			let right = rig.rotation(right_name) * Vec3::Y;
			assert!(left.x.abs() < 1e-3, "{left_name} left the sagittal plane: {left:?}");
			assert!(right.x.abs() < 1e-3, "{right_name} left the sagittal plane: {right:?}");
			assert!(
				(left.z - right.z).abs() < 1e-4,
				"{left_name} and {right_name} diverged: {left:?} vs {right:?}"
			);
			assert!(left.z > 0.2, "{left_name} should flex toward +Z, got {left:?}");
		}
	}
}
