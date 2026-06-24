use bevy::prelude::*;

use crate::{Name, RigPose, Side};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanoidLeg {
	pub pelvis: Name,
	pub femur: Name,
	pub shin: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanoidArm {
	pub shoulder: Name,
	pub humerus: Name,
	pub forearm: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanoidSpine {
	pub root: Name,
	pub lumbar: Name,
	pub midback: Name,
	pub upper_back: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanoidNeck {
	pub lower_neck: Name,
	pub upper_neck: Name,
}

pub trait HumanoidRig {
	fn leg(&self, side: Side) -> HumanoidLeg;
	fn arm(&self, side: Side) -> HumanoidArm;
	fn spine(&self) -> HumanoidSpine;
	fn neck(&self) -> HumanoidNeck;
	fn pose(&self) -> &RigPose;
	fn pose_mut(&mut self) -> &mut RigPose;

	fn pose_leg(&mut self, side: Side, pelvis: Transform, femur: Transform, shin: Transform) {
		let leg = self.leg(side);
		self.pose_mut().set_transform(leg.pelvis, pelvis);
		self.pose_mut().set_transform(leg.femur, femur);
		self.pose_mut().set_transform(leg.shin, shin);
	}

	fn pose_arm(
		&mut self,
		side: Side,
		shoulder: Transform,
		humerus: Transform,
		forearm: Transform,
	) {
		let arm = self.arm(side);
		self.pose_mut().set_transform(arm.shoulder, shoulder);
		self.pose_mut().set_transform(arm.humerus, humerus);
		self.pose_mut().set_transform(arm.forearm, forearm);
	}

	fn pose_spine(
		&mut self,
		root: Transform,
		lumbar: Transform,
		midback: Transform,
		upper_back: Transform,
	) {
		let spine = self.spine();
		self.pose_mut().set_transform(spine.root, root);
		self.pose_mut().set_transform(spine.lumbar, lumbar);
		self.pose_mut().set_transform(spine.midback, midback);
		self.pose_mut().set_transform(spine.upper_back, upper_back);
	}

	fn pose_neck(&mut self, lower_neck: Transform, upper_neck: Transform) {
		let neck = self.neck();
		self.pose_mut().set_transform(neck.lower_neck, lower_neck);
		self.pose_mut().set_transform(neck.upper_neck, upper_neck);
	}
}
