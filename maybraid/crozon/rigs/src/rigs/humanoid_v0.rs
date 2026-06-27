use bevy::prelude::*;
use std::collections::HashMap;

use crate::{
	articulation::{BoneArticulationFrame, FlexAxis},
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
	articulation_frames: HashMap<Name, BoneArticulationFrame>,
}

impl HumanoidV0Rig {
	pub fn imported() -> Self {
		let mut bones = BoneTable::new();
		for (name, relative_axis) in HUMANOID_V0_BONE_DEFINITIONS {
			bones.insert(BoneDefinition { name: Name::from(name), relative_axis });
		}

		let mut rig = Self {
			bones,
			pose: RigPose::new(),
			segment_lengths: LegSegmentLengths::default(),
			articulation_frames: HashMap::new(),
		};
		rig.install_articulation_frames();
		rig
	}

	/// Derive each bone's swing/flex axes statically from its `RiggedAxis` orientation.
	///
	/// No runtime probing: if a bone is exported with an unexpected orientation, encode the
	/// correction in its `RiggedAxis` entry and the articulation axes follow automatically.
	fn install_articulation_frames(&mut self) {
		for bone in self.animation_bones() {
			let axis = self.bones.get(&bone).map(|bone| bone.relative_axis).unwrap_or_default();
			let frame =
				BoneArticulationFrame::from_rigged_axis(axis, flex_axis_kind(bone.as_str()));
			self.articulation_frames.insert(bone, frame);
		}
	}
}

/// Map a bone's anatomical role to the plane it flexes in.
fn flex_axis_kind(bone: &str) -> FlexAxis {
	if bone.starts_with("shoulder") || bone.starts_with("pelvis") || bone.starts_with("humerus") {
		FlexAxis::Frontal
	} else {
		FlexAxis::Hinge
	}
}

impl HumanoidRig for HumanoidV0Rig {
	fn leg(&self, side: Side) -> HumanoidLeg {
		let suffix = side.suffix();
		HumanoidLeg {
			pelvis: BonePose::new(format!("pelvis.{suffix}"), Transform::IDENTITY),
			femur: BonePose::new(format!("femur.{suffix}"), Transform::IDENTITY),
			shin: BonePose::new(format!("shin.{suffix}"), Transform::IDENTITY),
		}
	}

	fn arm(&self, side: Side) -> HumanoidArm {
		let suffix = side.suffix();
		HumanoidArm {
			shoulder: BonePose::new(format!("shoulder.{suffix}"), Transform::IDENTITY),
			humerus: BonePose::new(format!("humerus.{suffix}"), Transform::IDENTITY),
			forearm: BonePose::new(format!("forearm.{suffix}"), Transform::IDENTITY),
		}
	}

	fn spine(&self) -> HumanoidSpine {
		HumanoidSpine {
			root: BonePose::new("root", Transform::IDENTITY),
			lumbar: BonePose::new("lumbar", Transform::IDENTITY),
			midback: BonePose::new("midback", Transform::IDENTITY),
			upper_back: BonePose::new("upper_back", Transform::IDENTITY),
		}
	}

	fn neck(&self) -> HumanoidNeck {
		HumanoidNeck {
			lower_neck: BonePose::new("lower_neck", Transform::IDENTITY),
			upper_neck: BonePose::new("upper_neck", Transform::IDENTITY),
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

	fn articulation_frame(&self, bone: &Name) -> Option<BoneArticulationFrame> {
		self.articulation_frames.get(bone).copied()
	}

	fn animation_bones(&self) -> Vec<Name> {
		HumanoidV0Rig::animation_bones(self)
	}

	fn segment_lengths(&self) -> LegSegmentLengths {
		self.segment_lengths
	}
}

fn flex_sign_from_axis(axis: RiggedAxis) -> f32 {
	if axis.forward.dot(RiggedAxis::DEFAULT.forward) < 0.0 {
		-1.0
	} else {
		1.0
	}
}

impl HumanoidV0Rig {
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
