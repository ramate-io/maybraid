use crate::{BonePose, RigPose, Side};

#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidLeg {
	pub pelvis: BonePose,
	pub femur: BonePose,
	pub shin: BonePose,
}

impl HumanoidLeg {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.pelvis);
		hydrate_bone(pose, &mut self.femur);
		hydrate_bone(pose, &mut self.shin);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.pelvis.clone());
		pose.insert(self.femur.clone());
		pose.insert(self.shin.clone());
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidArm {
	pub shoulder: BonePose,
	pub humerus: BonePose,
	pub forearm: BonePose,
}

impl HumanoidArm {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.shoulder);
		hydrate_bone(pose, &mut self.humerus);
		hydrate_bone(pose, &mut self.forearm);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.shoulder.clone());
		pose.insert(self.humerus.clone());
		pose.insert(self.forearm.clone());
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidSpine {
	pub root: BonePose,
	pub lumbar: BonePose,
	pub midback: BonePose,
	pub upper_back: BonePose,
}

impl HumanoidSpine {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.root);
		hydrate_bone(pose, &mut self.lumbar);
		hydrate_bone(pose, &mut self.midback);
		hydrate_bone(pose, &mut self.upper_back);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.root.clone());
		pose.insert(self.lumbar.clone());
		pose.insert(self.midback.clone());
		pose.insert(self.upper_back.clone());
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidNeck {
	pub lower_neck: BonePose,
	pub upper_neck: BonePose,
}

impl HumanoidNeck {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.lower_neck);
		hydrate_bone(pose, &mut self.upper_neck);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.lower_neck.clone());
		pose.insert(self.upper_neck.clone());
	}
}

fn hydrate_bone(pose: &RigPose, bone: &mut BonePose) {
	if let Some(stored) = pose.get(&bone.name) {
		bone.transform = stored.transform;
		bone.swing = stored.swing;
		bone.flex = stored.flex;
	}
}

pub trait HumanoidRig {
	fn leg(&self, side: Side) -> HumanoidLeg;
	fn arm(&self, side: Side) -> HumanoidArm;
	fn spine(&self) -> HumanoidSpine;
	fn neck(&self) -> HumanoidNeck;
	fn pose(&self) -> &RigPose;
	fn pose_mut(&mut self) -> &mut RigPose;

	/// +1 or −1 so mirrored forearms flex forward consistently in run animation.
	fn forearm_flex_sign(&self, _side: Side) -> f32 {
		1.0
	}

	fn leg_pose(&self, side: Side) -> HumanoidLeg {
		let mut leg = self.leg(side);
		leg.hydrate_from(self.pose());
		leg
	}

	fn arm_pose(&self, side: Side) -> HumanoidArm {
		let mut arm = self.arm(side);
		arm.hydrate_from(self.pose());
		arm
	}

	fn spine_pose(&self) -> HumanoidSpine {
		let mut spine = self.spine();
		spine.hydrate_from(self.pose());
		spine
	}

	fn neck_pose(&self) -> HumanoidNeck {
		let mut neck = self.neck();
		neck.hydrate_from(self.pose());
		neck
	}

	fn pose_leg(&mut self, leg: HumanoidLeg) {
		leg.apply_to(self.pose_mut());
	}

	fn pose_arm(&mut self, arm: HumanoidArm) {
		arm.apply_to(self.pose_mut());
	}

	fn pose_spine(&mut self, spine: HumanoidSpine) {
		spine.apply_to(self.pose_mut());
	}

	fn pose_neck(&mut self, neck: HumanoidNeck) {
		neck.apply_to(self.pose_mut());
	}
}
