use bevy::prelude::*;

use crate::{BonePose, Name, RigPose, RiggedAxis, Side};

/// Rest-pose upper and lower leg segment lengths for analytic IK-style drop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegSegmentLengths {
	pub upper: f32,
	pub lower: f32,
}

impl Default for LegSegmentLengths {
	fn default() -> Self {
		Self { upper: 0.5, lower: 0.5 }
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuadrupedFrontLeg {
	pub shoulder: BonePose,
	pub thigh: BonePose,
	pub shin: BonePose,
}

impl QuadrupedFrontLeg {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.shoulder);
		hydrate_bone(pose, &mut self.thigh);
		hydrate_bone(pose, &mut self.shin);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.shoulder.clone());
		pose.insert(self.thigh.clone());
		pose.insert(self.shin.clone());
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuadrupedHindLeg {
	pub hip: BonePose,
	pub thigh: BonePose,
	pub shin: BonePose,
}

impl QuadrupedHindLeg {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.hip);
		hydrate_bone(pose, &mut self.thigh);
		hydrate_bone(pose, &mut self.shin);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.hip.clone());
		pose.insert(self.thigh.clone());
		pose.insert(self.shin.clone());
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuadrupedSpine {
	pub back_ridge: BonePose,
	pub upper_back: BonePose,
	pub lumbar: BonePose,
}

impl QuadrupedSpine {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.back_ridge);
		hydrate_bone(pose, &mut self.upper_back);
		hydrate_bone(pose, &mut self.lumbar);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.back_ridge.clone());
		pose.insert(self.upper_back.clone());
		pose.insert(self.lumbar.clone());
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuadrupedNeck {
	pub neck: BonePose,
}

impl QuadrupedNeck {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.neck);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.neck.clone());
	}
}

fn hydrate_bone(pose: &RigPose, bone: &mut BonePose) {
	if let Some(stored) = pose.get(&bone.name) {
		bone.transform = stored.transform;
		bone.swing = stored.swing;
		bone.flex = stored.flex;
		bone.twist = stored.twist;
	}
}

pub trait QuadrupedRig {
	fn front_leg(&self, side: Side) -> QuadrupedFrontLeg;
	fn hind_leg(&self, side: Side) -> QuadrupedHindLeg;
	fn spine(&self) -> QuadrupedSpine;
	fn neck(&self) -> QuadrupedNeck;
	fn pose(&self) -> &RigPose;
	fn pose_mut(&mut self) -> &mut RigPose;

	fn animation_bones(&self) -> Vec<Name>;

	fn segment_lengths(&self) -> LegSegmentLengths {
		LegSegmentLengths::default()
	}

	fn shin_flex_sign(&self, _side: Side, _front: bool) -> f32 {
		1.0
	}

	fn rigged_axis(&self, _bone: &Name) -> Option<RiggedAxis> {
		None
	}

	fn articulate_on_rig(&self, mut bone: BonePose, swing: f32, flex: f32) -> BonePose {
		if let Some(axis) = self.rigged_axis(&bone.name) {
			bone = bone.articulate(axis, swing, flex, 0.0);
		} else {
			bone.swing = swing;
			bone.flex = flex;
		}
		bone
	}

	fn front_leg_pose(&self, side: Side) -> QuadrupedFrontLeg {
		let mut leg = self.front_leg(side);
		leg.hydrate_from(self.pose());
		leg
	}

	fn hind_leg_pose(&self, side: Side) -> QuadrupedHindLeg {
		let mut leg = self.hind_leg(side);
		leg.hydrate_from(self.pose());
		leg
	}

	fn spine_pose(&self) -> QuadrupedSpine {
		let mut spine = self.spine();
		spine.hydrate_from(self.pose());
		spine
	}

	fn neck_pose(&self) -> QuadrupedNeck {
		let mut neck = self.neck();
		neck.hydrate_from(self.pose());
		neck
	}

	fn pose_front_leg(&mut self, leg: QuadrupedFrontLeg) {
		leg.apply_to(self.pose_mut());
	}

	fn pose_hind_leg(&mut self, leg: QuadrupedHindLeg) {
		leg.apply_to(self.pose_mut());
	}

	fn pose_spine(&mut self, spine: QuadrupedSpine) {
		spine.apply_to(self.pose_mut());
	}

	fn pose_neck(&mut self, neck: QuadrupedNeck) {
		neck.apply_to(self.pose_mut());
	}

	fn parent_world_rotation(&self, _bone: &Name) -> Quat {
		Quat::IDENTITY
	}

	fn move_all(&mut self, world_displacement: Vec3) {
		let bones = self.animation_bones();
		for bone in bones {
			let parent_world = self.parent_world_rotation(&bone);
			let axis = self.rigged_axis(&bone).unwrap_or(RiggedAxis::DEFAULT);
			let Some(pose) = self.pose_mut().get_mut(&bone) else {
				continue;
			};
			let segment = pose.transform.translation;
			let delta = crate::articulation::axis_aware_translation_delta(
				segment,
				axis,
				world_displacement,
				parent_world,
			);
			pose.transform.translation += delta;
		}
	}
}
