use bevy::prelude::*;

use crate::{
	humanoid::{
		HumanoidArm, HumanoidLeg, HumanoidNeck, HumanoidRig, HumanoidSpine, LegSegmentLengths,
	},
	BoneDefinition, BonePose, BoneTable, Name, RigPose, RiggedAxis, Side,
};

/// Store the bones of the first imported humanoid rig in a semantically reasonable hierarchy.
///
/// Symmetry is currently represented by explicit accessors (`arm(Side)`, `leg(Side)`) rather
/// than a generic table. That keeps rig-specific mirror relationships local until repeated
/// patterns across humanoids and quadrupeds make a shared abstraction worth adding.
#[derive(Component, Debug, Clone)]
pub struct HumanoidV0Rig {
	pub bones: BoneTable,
	pub pose: RigPose,
	pub segment_lengths: LegSegmentLengths,
}

impl HumanoidV0Rig {
	pub fn imported() -> Self {
		let mut bones = BoneTable::new();
		for (name, relative_axis) in HUMANOID_V0_BONE_DEFINITIONS {
			bones.insert(BoneDefinition { name: Name::from(name), relative_axis });
		}

		Self { bones, pose: RigPose::new(), segment_lengths: LegSegmentLengths::default() }
	}
}

impl HumanoidRig for HumanoidV0Rig {
	fn leg(&self, side: Side) -> HumanoidLeg {
		let suffix = side.suffix();
		HumanoidLeg {
			pelvis: self.bone_pose(format!("pelvis.{suffix}")),
			femur: self.bone_pose(format!("femur.{suffix}")),
			shin: self.bone_pose(format!("shin.{suffix}")),
		}
	}

	fn arm(&self, side: Side) -> HumanoidArm {
		let suffix = side.suffix();
		HumanoidArm {
			shoulder: self.bone_pose(format!("shoulder.{suffix}")),
			humerus: self.bone_pose(format!("humerus.{suffix}")),
			forearm: self.bone_pose(format!("forearm.{suffix}")),
		}
	}

	fn spine(&self) -> HumanoidSpine {
		HumanoidSpine {
			root: self.bone_pose("root"),
			lumbar: self.bone_pose("lumbar"),
			midback: self.bone_pose("midback"),
			upper_back: self.bone_pose("upper_back"),
		}
	}

	fn neck(&self) -> HumanoidNeck {
		HumanoidNeck {
			lower_neck: self.bone_pose("lower_neck"),
			upper_neck: self.bone_pose("upper_neck"),
		}
	}

	fn pose(&self) -> &RigPose {
		&self.pose
	}

	fn pose_mut(&mut self) -> &mut RigPose {
		&mut self.pose
	}

	fn forearm_flex_sign(&self, side: Side) -> f32 {
		let name = Name::from(format!("forearm.{}", side.suffix()));
		self.bones
			.get(&name)
			.map(|bone| flex_sign_from_axis(bone.relative_axis))
			.unwrap_or(1.0)
	}

	fn rigged_axis(&self, bone: &Name) -> Option<RiggedAxis> {
		self.bones.get(bone).map(|bone| bone.relative_axis)
	}

	fn animation_bones(&self) -> Vec<Name> {
		HumanoidV0Rig::animation_bones(self)
	}

	fn segment_lengths(&self) -> LegSegmentLengths {
		self.segment_lengths
	}
}

fn flex_sign_from_axis(axis: RiggedAxis) -> f32 {
	if axis.flex_axis.dot(RiggedAxis::DEFAULT.flex_axis) < 0.0 {
		-1.0
	} else {
		1.0
	}
}

impl HumanoidV0Rig {
	fn bone_pose(&self, name: impl Into<Name>) -> BonePose {
		let name = name.into();
		self.pose
			.get(&name)
			.cloned()
			.unwrap_or_else(|| BonePose::new(name, Transform::IDENTITY))
	}

	pub fn animation_bones(&self) -> Vec<Name> {
		let left_arm = self.arm(Side::Left);
		let right_arm = self.arm(Side::Right);
		let left_leg = self.leg(Side::Left);
		let right_leg = self.leg(Side::Right);
		let root = self.spine().root;

		vec![
			root.name,
			left_arm.shoulder.name,
			right_arm.shoulder.name,
			left_arm.humerus.name,
			left_arm.forearm.name,
			right_arm.humerus.name,
			right_arm.forearm.name,
			left_leg.pelvis.name,
			right_leg.pelvis.name,
			left_leg.femur.name,
			left_leg.shin.name,
			right_leg.femur.name,
			right_leg.shin.name,
		]
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
	("forearm.R", RiggedAxis::DEFAULT),
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
	("femur.L", RiggedAxis::DEFAULT),
	("shin.L", RiggedAxis::SHIN),
	("calf_thickness.L", RiggedAxis::DEFAULT),
	("thigh_thickness.L", RiggedAxis::DEFAULT),
	("pelvis.R", RiggedAxis::DEFAULT),
	("femur.R", RiggedAxis::DEFAULT),
	("shin.R", RiggedAxis::SHIN),
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

	#[test]
	fn humanoid_v0_default_segment_lengths() {
		let rig = HumanoidV0Rig::imported();
		assert_eq!(rig.segment_lengths, LegSegmentLengths::default());
	}

	#[test]
	fn humanoid_v0_move_all_updates_animation_bones() {
		let mut rig = HumanoidV0Rig::imported();
		rig.pose_leg({
			let mut leg = rig.leg(Side::Left);
			leg.femur = BonePose::with_articulation(leg.femur.name, 0.25, 0.0);
			leg
		});
		rig.move_all(Vec3::new(0.0, -0.15, 0.0));

		let femur = rig.pose().get(&Name::from("femur.L")).expect("femur pose");
		assert_eq!(femur.swing, 0.25);
		assert_eq!(femur.transform.translation, Vec3::new(0.0, -0.15, 0.0));

		let shoulder = rig.pose().get(&Name::from("shoulder.L")).expect("shoulder pose");
		assert_eq!(shoulder.transform.translation, Vec3::new(0.0, -0.15, 0.0));
	}

	#[test]
	fn humanoid_v0_accessors_map_to_imported_names() {
		let rig = HumanoidV0Rig::imported();

		assert_eq!(rig.leg(Side::Left).femur.name, Name::from("femur.L"));
		assert_eq!(rig.leg(Side::Right).shin.name, Name::from("shin.R"));
		assert_eq!(rig.arm(Side::Left).humerus.name, Name::from("humerus.L"));
		assert_eq!(rig.arm(Side::Right).forearm.name, Name::from("forearm.R"));
		assert_eq!(rig.neck().upper_neck.name, Name::from("upper_neck"));
	}

	#[test]
	fn humanoid_v0_animation_bones_exist_in_definition_table() {
		let rig = HumanoidV0Rig::imported();

		for name in rig.animation_bones() {
			assert!(rig.bones.get(&name).is_some(), "missing animation bone {name}");
		}
	}

	#[test]
	fn humanoid_v0_uses_default_semantic_pose_writers() {
		let mut rig = HumanoidV0Rig::imported();
		let mut leg = rig.leg(Side::Left);
		let mut arm = rig.arm(Side::Right);
		let femur = Transform::from_translation(Vec3::X);
		let forearm = Transform::from_translation(Vec3::Y);

		leg.femur.transform = femur;
		arm.forearm.transform = forearm;
		rig.pose_leg(leg);
		rig.pose_arm(arm);

		assert_eq!(rig.pose().get(&Name::from("femur.L")).map(|pose| pose.transform), Some(femur));
		assert_eq!(
			rig.pose().get(&Name::from("forearm.R")).map(|pose| pose.transform),
			Some(forearm)
		);
	}

	#[test]
	fn humanoid_v0_leg_pose_round_trips_through_rig_pose() {
		let mut rig = HumanoidV0Rig::imported();
		let mut leg = rig.leg(Side::Left);
		leg.shin.transform = Transform::from_translation(Vec3::Z);

		rig.pose_leg(leg);
		let hydrated = rig.leg_pose(Side::Left);

		assert_eq!(hydrated.shin.transform, Transform::from_translation(Vec3::Z));
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
