//! Simply defines a hysteresis rule from one point to another.

use crate::BallStickNode;
use crate::Hysteresis;
use bevy_math::Vec3;

#[derive(Clone, Debug)]
pub struct PointToPoint {
	pub start: BallStickNode,
	pub end: Option<BallStickNode>,
	pub radius: f32,
}

impl PointToPoint {
	pub fn new(start: BallStickNode, end: BallStickNode, radius: f32) -> Self {
		Self { start, end: Some(end), radius }
	}

	pub fn new_from_vec3(start: Vec3, end: Vec3, radius: f32) -> Self {
		Self::new(BallStickNode::new(start, radius), BallStickNode::new(end, radius), radius)
	}
}

impl Hysteresis for PointToPoint {
	fn ball_stick_node(&self) -> BallStickNode {
		self.start
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		// consume the end node
		if let Some(end) = self.end {
			vec![Self { start: end, end: None, radius: self.radius }]
		} else {
			vec![]
		}
	}
}
