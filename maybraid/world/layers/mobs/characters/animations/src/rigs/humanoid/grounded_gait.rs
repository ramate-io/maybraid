use crate::animations::{GroundedGait, Idle, Mix, Run, Walk};
use crate::{Animation, Effects};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

impl Animation<HumanoidV0Rig> for GroundedGait {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		Mix::new(
			Idle::default(),
			Mix::new(Walk::default(), Run::default(), self.run_weight),
			self.walk_weight,
		)
		.apply_for(rig, progress);
	}
	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn weights_reach_idle_walk_and_run_and_nested_blends_keep_the_outer_capture(
	) -> anyhow::Result<()> {
		let mut idle = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut idle, 0.31);
		let mut walk = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut walk, 0.31);
		let mut run = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut run, 0.31);
		for (weights, expected) in [((0.0, 0.7), idle), ((1.0, 0.0), walk), ((1.0, 1.0), run)] {
			let mut actual = HumanoidV0Rig::for_clip_test();
			GroundedGait::new(weights.0, weights.1).apply(&mut actual, 0.31);
			for (a, b) in actual.pose.local.iter().zip(&expected.pose.local) {
				assert!(a.translation.distance(b.translation) < 1e-6);
				assert!(a.rotation.normalize().dot(b.rotation.normalize()).abs() > 1.0 - 1e-6);
			}
			assert_eq!(actual.scratch.depth, 0);
		}
		let mut actual = HumanoidV0Rig::for_clip_test();
		let original = actual.pose.clone();
		Mix::new(Idle::default(), GroundedGait::new(0.7, 0.3), 0.0).apply(&mut actual, 0.31);
		let mut expected = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut expected, 0.31);
		assert_eq!(actual.pose, expected.pose);
		assert_ne!(actual.pose, original);
		assert_eq!(actual.scratch.depth, 0);
		Ok(())
	}
}
