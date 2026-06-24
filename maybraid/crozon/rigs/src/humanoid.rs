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
}
