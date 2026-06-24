use crate::Name;

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
