use bevy::prelude::*;

use crate::{
	humanoid::{HumanoidArm, HumanoidLeg, HumanoidNeck, HumanoidRig, HumanoidSpine},
	BoneDefinition, BoneTable, Name, RigPose, RiggedAxis, Side,
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
}

impl HumanoidV0Rig {
	pub fn imported() -> Self {
		let mut bones = BoneTable::new();
		for (name, relative_axis) in HUMANOID_V0_BONE_DEFINITIONS {
			bones.insert(BoneDefinition { name: Name::from(name), relative_axis });
		}

		Self { bones, pose: RigPose::new() }
	}
}

impl HumanoidRig for HumanoidV0Rig {
	fn leg(&self, side: Side) -> HumanoidLeg {
		let suffix = side.suffix();
		HumanoidLeg {
			pelvis: Name::new(format!("pelvis.{suffix}")),
			femur: Name::new(format!("femur.{suffix}")),
			shin: Name::new(format!("shin.{suffix}")),
		}
	}

	fn arm(&self, side: Side) -> HumanoidArm {
		let suffix = side.suffix();
		HumanoidArm {
			shoulder: Name::new(format!("shoulder.{suffix}")),
			humerus: Name::new(format!("humerus.{suffix}")),
			forearm: Name::new(format!("forearm.{suffix}")),
		}
	}

	fn spine(&self) -> HumanoidSpine {
		HumanoidSpine {
			root: Name::from("root"),
			lumbar: Name::from("lumbar"),
			midback: Name::from("midback"),
			upper_back: Name::from("upper_back"),
		}
	}

	fn neck(&self) -> HumanoidNeck {
		HumanoidNeck { lower_neck: Name::from("lower_neck"), upper_neck: Name::from("upper_neck") }
	}

	fn pose(&self) -> &RigPose {
		&self.pose
	}

	fn pose_mut(&mut self) -> &mut RigPose {
		&mut self.pose
	}
}

impl HumanoidV0Rig {
	pub fn animation_bones(&self) -> Vec<Name> {
		let left_arm = self.arm(Side::Left);
		let right_arm = self.arm(Side::Right);
		let left_leg = self.leg(Side::Left);
		let right_leg = self.leg(Side::Right);

		vec![
			left_arm.shoulder,
			right_arm.shoulder,
			left_arm.humerus,
			left_arm.forearm,
			right_arm.humerus,
			right_arm.forearm,
			left_leg.pelvis,
			right_leg.pelvis,
			left_leg.femur,
			left_leg.shin,
			right_leg.femur,
			right_leg.shin,
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
	("shin.L", RiggedAxis::DEFAULT),
	("calf_thickness.L", RiggedAxis::DEFAULT),
	("thigh_thickness.L", RiggedAxis::DEFAULT),
	("pelvis.R", RiggedAxis::DEFAULT),
	("femur.R", RiggedAxis::DEFAULT),
	("shin.R", RiggedAxis::DEFAULT),
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
	fn humanoid_v0_accessors_map_to_imported_names() {
		let rig = HumanoidV0Rig::imported();

		assert_eq!(rig.leg(Side::Left).femur, Name::from("femur.L"));
		assert_eq!(rig.leg(Side::Right).shin, Name::from("shin.R"));
		assert_eq!(rig.arm(Side::Left).humerus, Name::from("humerus.L"));
		assert_eq!(rig.arm(Side::Right).forearm, Name::from("forearm.R"));
		assert_eq!(rig.neck().upper_neck, Name::from("upper_neck"));
	}

	#[test]
	fn humanoid_v0_animation_bones_exist_in_definition_table() {
		let rig = HumanoidV0Rig::imported();

		for name in rig.animation_bones() {
			assert!(rig.bones.get(&name).is_some(), "missing animation bone {name}");
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
