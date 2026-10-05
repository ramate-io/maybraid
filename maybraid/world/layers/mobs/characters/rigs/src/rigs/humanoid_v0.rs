use bevy::prelude::*;

use crate::{
	authoring::{
		humanoid_v0_definition, resolve_humanoid, HumanoidPose, PoseBuffer, PoseScratch,
		RigBinding, HUMANOID_FEMUR_AXIS, HUMANOID_RIGHT_FEMUR_AXIS, HUMANOID_RIGHT_FLEX_AXIS,
		HUMANOID_RIGHT_SHIN_AXIS, HUMANOID_SHIN_AXIS, HUMANOID_V0_BONES,
	},
	humanoid::LegSegmentLengths,
	BoneDefinition, BoneTable, Name, RiggedAxis,
};

/// Store the bones of the first imported humanoid rig in a semantically reasonable hierarchy.
///
/// Symmetry is currently represented by explicit accessors (`arm(Side)`, `leg(Side)`) rather
/// than a generic table. That keeps rig-specific mirror relationships local until repeated
/// patterns across humanoids and quadrupeds make a shared abstraction worth adding.
#[derive(Component, Debug, Clone)]
pub struct HumanoidV0Rig {
	pub bones: BoneTable,
	pub binding: RigBinding,
	/// Last resolved local pose. Sampling always starts from [`Self::binding`] rest.
	pub pose: PoseBuffer,
	pub scratch: PoseScratch,
	pub segment_lengths: LegSegmentLengths,
}

impl HumanoidV0Rig {
	/// Identity rest plus the inspected `humanoid_rig.glb` limb binds.
	pub fn for_clip_test() -> Self {
		let mut rig = Self::imported();
		rig.seed_v0_bind();
		rig
	}

	pub fn imported() -> Self {
		let mut bones = BoneTable::new();
		for (name, relative_axis) in HUMANOID_V0_BONE_DEFINITIONS {
			bones.insert(BoneDefinition { name: Name::from(name), relative_axis });
		}
		let definition = humanoid_v0_definition();
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

	pub fn write_pose(&mut self, pose: &HumanoidPose) {
		resolve_humanoid(pose, &self.binding, &mut self.pose);
		self.segment_lengths = self.binding.metrics.humanoid_leg;
	}

	/// Load the inspected `humanoid_rig.glb` bind rotations used by authoring tests.
	pub fn seed_v0_bind(&mut self) {
		let mut rest = self.binding.effective_rest.clone();
		let set = |rest: &mut PoseBuffer, name: &str, rotation: Quat| {
			if let Some(id) = self.binding.definition.id(name) {
				if let Some(slot) = rest.local.get_mut(id.index()) {
					slot.rotation = rotation;
				}
			}
		};
		set(&mut rest, "pelvis.L", crate::authoring::HUMANOID_GLB_PELVIS_L);
		set(&mut rest, "pelvis.R", crate::authoring::HUMANOID_GLB_PELVIS_R);
		set(&mut rest, "femur.L", crate::authoring::HUMANOID_GLB_FEMUR);
		set(&mut rest, "femur.R", crate::authoring::HUMANOID_GLB_FEMUR);
		set(&mut rest, "shoulder.L", crate::authoring::HUMANOID_GLB_SHOULDER_L);
		set(&mut rest, "shoulder.R", crate::authoring::HUMANOID_GLB_SHOULDER_R);
		self.binding.refresh_rest(rest);
		self.pose.copy_from(&self.binding.effective_rest);
		self.segment_lengths = self.binding.metrics.humanoid_leg;
	}

	/// Bone +Y in character space (parents included).
	pub fn character_length(&self, name: &str) -> Vec3 {
		self.binding
			.definition
			.id(name)
			.map(|id| self.binding.definition.rotation_in_character(&self.pose, id) * Vec3::Y)
			.unwrap_or(Vec3::Y)
	}

	/// Replace one bone's effective rest and recalibrate frames.
	pub fn seed_rest(&mut self, name: &str, transform: Transform) {
		let Some(id) = self.binding.definition.id(name) else {
			return;
		};
		let mut rest = self.binding.effective_rest.clone();
		if let Some(slot) = rest.local.get_mut(id.index()) {
			*slot = transform;
		}
		self.binding.refresh_rest(rest);
		self.segment_lengths = self.binding.metrics.humanoid_leg;
	}

	/// Angle between the posed local rotation and effective rest.
	pub fn posed_angle(&self, name: &str) -> f32 {
		let Some(id) = self.binding.definition.id(name) else {
			return 0.0;
		};
		self.pose.rotation(id).angle_between(self.binding.effective_rest.rotation(id))
	}

	pub fn rotation(&self, name: &str) -> Quat {
		self.binding
			.definition
			.id(name)
			.map(|id| self.pose.rotation(id))
			.unwrap_or(Quat::IDENTITY)
	}

	pub fn animation_bone_names(&self) -> impl Iterator<Item = &'static str> {
		HUMANOID_V0_BONES.iter().copied()
	}
}

impl HumanoidV0Rig {
	pub fn rigged_axis(&self, bone: &Name) -> Option<RiggedAxis> {
		self.bones.get(bone).map(|bone| bone.relative_axis)
	}

	pub fn animation_bones(&self) -> Vec<Name> {
		self.animation_bone_names().map(Name::from).collect()
	}
}

impl Default for HumanoidV0Rig {
	fn default() -> Self {
		Self::imported()
	}
}

pub const HUMANOID_V0_BONE_DEFINITIONS: [(&str, RiggedAxis); 37] = [
	("root", RiggedAxis::DEFAULT),
	("lumbar", RiggedAxis::DEFAULT),
	("midback", RiggedAxis::DEFAULT),
	("upper_back", RiggedAxis::DEFAULT),
	("shoulder.L", RiggedAxis::DEFAULT),
	("humerus.L", RiggedAxis::DEFAULT),
	("forearm.L", RiggedAxis::DEFAULT),
	("lower_arm_thickness.L", RiggedAxis::DEFAULT),
	("upper_arm_thickness.L", RiggedAxis::DEFAULT),
	("lower_neck", RiggedAxis::DEFAULT),
	("upper_neck", RiggedAxis::DEFAULT),
	("shoulder.R", RiggedAxis::DEFAULT),
	("humerus.R", RiggedAxis::DEFAULT),
	("forearm.R", HUMANOID_RIGHT_FLEX_AXIS),
	("lower_arm_thickness.R", RiggedAxis::DEFAULT),
	("upper_arm_thickness.R", RiggedAxis::DEFAULT),
	("chest.L", RiggedAxis::DEFAULT),
	("chest.R", RiggedAxis::DEFAULT),
	("upper_back_thickness", RiggedAxis::DEFAULT),
	("chest_thickness", RiggedAxis::DEFAULT),
	("lat.L", RiggedAxis::DEFAULT),
	("lat.R", RiggedAxis::DEFAULT),
	("upper_belly", RiggedAxis::DEFAULT),
	("waist.L", RiggedAxis::DEFAULT),
	("waist.R", RiggedAxis::DEFAULT),
	("lower_belly", RiggedAxis::DEFAULT),
	("pelvis.L", RiggedAxis::DEFAULT),
	("femur.L", HUMANOID_FEMUR_AXIS),
	("shin.L", HUMANOID_SHIN_AXIS),
	("calf_thickness.L", RiggedAxis::DEFAULT),
	("thigh_thickness.L", RiggedAxis::DEFAULT),
	("pelvis.R", RiggedAxis::DEFAULT),
	("femur.R", HUMANOID_RIGHT_FEMUR_AXIS),
	("shin.R", HUMANOID_RIGHT_SHIN_AXIS),
	("calf_thickness.R", RiggedAxis::DEFAULT),
	("thigh_thickness.R", RiggedAxis::DEFAULT),
	("buttocks", RiggedAxis::DEFAULT),
];

pub fn humanoid_v0_bone_names() -> impl Iterator<Item = &'static str> {
	HUMANOID_V0_BONE_DEFINITIONS.into_iter().map(|(name, _axis)| name)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::authoring::HumanoidPose;
	use crate::Side;

	#[test]
	fn humanoid_v0_default_segment_lengths() {
		let rig = HumanoidV0Rig::imported();
		assert_eq!(rig.segment_lengths, LegSegmentLengths::default());
	}

	#[test]
	fn proportion_edit_refreshes_cached_leg_length() {
		let mut rig = HumanoidV0Rig::imported();
		rig.seed_rest("femur.L", Transform::from_translation(Vec3::Y * 0.8));
		rig.seed_rest("shin.L", Transform::from_translation(Vec3::Y * 0.6));
		assert!((rig.segment_lengths.femur - 0.8).abs() < 1e-5);
		assert!((rig.segment_lengths.shin - 0.6).abs() < 1e-5);
		let mut pose = HumanoidPose::default();
		pose.leg_mut(Side::Left).hip_flexion = 0.2;
		rig.write_pose(&pose);
		assert!((rig.segment_lengths.femur - 0.8).abs() < 1e-5);
	}

	#[test]
	fn humanoid_v0_animation_bones_exist_in_definition_table() {
		let rig = HumanoidV0Rig::imported();
		for name in rig.animation_bones() {
			assert!(rig.bones.get(&name).is_some(), "missing animation bone {name}");
			assert!(rig.binding.definition.id(name.as_str()).is_some(), "missing id {name}");
		}
	}

	#[test]
	fn humanoid_v0_definition_covers_imported_dump() {
		let rig = HumanoidV0Rig::imported();
		for name in humanoid_v0_bone_names() {
			assert!(rig.bones.get(&Name::from(name)).is_some(), "missing bone {name}");
		}
		assert_eq!(rig.bones.len(), HUMANOID_V0_BONE_DEFINITIONS.len());
	}
}
