use bevy::prelude::*;

use crate::{
	quadruped::{
		QuadrupedFrontLeg, QuadrupedHindLeg, QuadrupedNeck, QuadrupedRig, QuadrupedSpine,
		LegSegmentLengths,
	},
	BoneDefinition, BonePose, BoneTable, Name, RigPose, RiggedAxis, Side,
};

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
	pub pose: RigPose,
	pub segment_lengths: LegSegmentLengths,
}

impl QuadrupedV0Rig {
	pub fn imported() -> Self {
		let mut bones = BoneTable::new();
		for (name, relative_axis) in QUADRUPED_V0_BONE_DEFINITIONS {
			bones.insert(BoneDefinition { name: Name::from(name), relative_axis });
		}

		Self { bones, pose: RigPose::new(), segment_lengths: LegSegmentLengths::default() }
	}
}

impl QuadrupedRig for QuadrupedV0Rig {
	fn front_leg(&self, side: Side) -> QuadrupedFrontLeg {
		let suffix = side.suffix();
		QuadrupedFrontLeg {
			shoulder: self.bone_pose(format!("shoulder.{suffix}")),
			thigh: self.bone_pose(format!("anterior_thigh.{suffix}")),
			shin: self.bone_pose(format!("anterior_shin.{suffix}")),
		}
	}

	fn hind_leg(&self, side: Side) -> QuadrupedHindLeg {
		let suffix = side.suffix();
		QuadrupedHindLeg {
			hip: self.bone_pose(format!("hip.{suffix}")),
			thigh: self.bone_pose(format!("posterior_thigh.{suffix}")),
			shin: self.bone_pose(format!("posterior_shin.{suffix}")),
		}
	}

	fn spine(&self) -> QuadrupedSpine {
		QuadrupedSpine {
			back_ridge: self.bone_pose("back_ridge"),
			upper_back: self.bone_pose("upper_back"),
			lumbar: self.bone_pose("lumbar"),
		}
	}

	fn neck(&self) -> QuadrupedNeck {
		QuadrupedNeck { neck: self.bone_pose("neck") }
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
		QuadrupedV0Rig::animation_bones(self)
	}

	fn segment_lengths(&self) -> LegSegmentLengths {
		self.segment_lengths
	}

	fn parent_world_rotation(&self, bone: &Name) -> Quat {
		self.parent_world_rotation_for(bone)
	}
}

impl QuadrupedV0Rig {
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
		quadruped_v0_parent(bone.as_str())
			.map(|parent| self.world_rotation_for(&Name::from(parent)))
			.unwrap_or(Quat::IDENTITY)
	}

	pub fn animation_bones(&self) -> Vec<Name> {
		let left_front = self.front_leg(Side::Left);
		let right_front = self.front_leg(Side::Right);
		let left_hind = self.hind_leg(Side::Left);
		let right_hind = self.hind_leg(Side::Right);
		let spine = self.spine();
		let neck = self.neck();

		vec![
			spine.back_ridge.name,
			spine.upper_back.name,
			spine.lumbar.name,
			neck.neck.name,
			left_front.shoulder.name,
			right_front.shoulder.name,
			left_front.thigh.name,
			left_front.shin.name,
			right_front.thigh.name,
			right_front.shin.name,
			left_hind.hip.name,
			right_hind.hip.name,
			left_hind.thigh.name,
			left_hind.shin.name,
			right_hind.thigh.name,
			right_hind.shin.name,
		]
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

const QUADRUPED_V0_PARENT: &[(&str, &str)] = &[
	("back_ridge", ""),
	("upper_back", "back_ridge"),
	("lumbar", "back_ridge"),
	("neck", "upper_back"),
	("shoulder.L", "upper_back"),
	("shoulder.R", "upper_back"),
	("anterior_thigh.L", "shoulder.L"),
	("anterior_thigh.R", "shoulder.R"),
	("anterior_shin.L", "anterior_thigh.L"),
	("anterior_shin.R", "anterior_thigh.R"),
	("hip.L", "lumbar"),
	("hip.R", "lumbar"),
	("posterior_thigh.L", "hip.L"),
	("posterior_thigh.R", "hip.R"),
	("posterior_shin.L", "posterior_thigh.L"),
	("posterior_shin.R", "posterior_thigh.R"),
];

fn quadruped_v0_parent(name: &str) -> Option<&'static str> {
	QUADRUPED_V0_PARENT
		.iter()
		.find(|(child, _)| *child == name)
		.map(|(_, parent)| *parent)
		.filter(|parent| !parent.is_empty())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn quadruped_v0_accessors_map_to_imported_names() {
		let rig = QuadrupedV0Rig::imported();

		assert_eq!(rig.front_leg(Side::Left).thigh.name, Name::from("anterior_thigh.L"));
		assert_eq!(rig.hind_leg(Side::Right).shin.name, Name::from("posterior_shin.R"));
		assert_eq!(rig.spine().upper_back.name, Name::from("upper_back"));
		assert_eq!(rig.neck().neck.name, Name::from("neck"));
	}

	#[test]
	fn quadruped_v0_animation_bones_exist_in_definition_table() {
		let rig = QuadrupedV0Rig::imported();

		for name in rig.animation_bones() {
			assert!(rig.bones.get(&name).is_some(), "missing animation bone {name}");
		}
	}

	#[test]
	fn quadruped_v0_leg_pose_round_trips_through_rig_pose() {
		let mut rig = QuadrupedV0Rig::imported();
		let mut leg = rig.front_leg(Side::Left);
		leg.shin.transform = Transform::from_translation(Vec3::Z);

		rig.pose_front_leg(leg);
		let hydrated = rig.front_leg_pose(Side::Left);

		assert_eq!(hydrated.shin.transform, Transform::from_translation(Vec3::Z));
	}
}
