/// Continuous grounded locomotion. One phase drives both gait loops; retuning weights
/// changes pose composition without restarting the mailbox clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundedGait {
	pub walk_weight: f32,
	pub run_weight: f32,
}
impl GroundedGait {
	pub fn new(walk_weight: f32, run_weight: f32) -> Self {
		Self { walk_weight: walk_weight.clamp(0.0, 1.0), run_weight: run_weight.clamp(0.0, 1.0) }
	}
}
impl Default for GroundedGait {
	fn default() -> Self {
		Self::new(1.0, 0.0)
	}
}
