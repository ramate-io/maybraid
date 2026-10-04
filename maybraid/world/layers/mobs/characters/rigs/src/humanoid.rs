/// Rest-pose thigh and shin lengths used for analytic drop.
///
/// Sampling derives these from the effective rest. A translation whose length is
/// near zero keeps the default 0.5 m.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegSegmentLengths {
	pub femur: f32,
	pub shin: f32,
}

impl Default for LegSegmentLengths {
	fn default() -> Self {
		Self { femur: 0.5, shin: 0.5 }
	}
}
