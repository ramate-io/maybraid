use bevy::prelude::Quat;
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{Idle, Prone, ProneAscent};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for ProneAscent {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		blend_clips(rig, &Prone::default(), 1.0, &Idle::default(), 0.0, 1.0 - self.depth(progress));
		let push = self.arm_push(progress);
		if push > 0.0 {
			for name in ["humerus.L", "humerus.R"] {
				let Some(id) = rig.binding.definition.id(name) else {
					continue;
				};
				let current = rig.pose.rotation(id);
				rig.pose.set_rotation(id, Quat::from_rotation_y(push) * current);
			}
		}
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		ArmatureOffset::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::Transition;

	fn max_rotation_delta(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).normalize().angle_between(b.rotation(name).normalize()))
			.fold(0.0, f32::max)
	}

	fn sample(progress: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneAscent.apply(&mut rig, progress);
		rig
	}

	#[test]
	fn endpoints_match_held_prone_and_idle_zero() -> anyhow::Result<()> {
		let start = sample(0.0);
		let end = sample(1.0);
		let mut prone = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut prone, 1.0);
		let mut idle = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut idle, 0.0);
		assert!(max_rotation_delta(&start, &prone) < 1e-5);
		assert!(max_rotation_delta(&end, &idle) < 1e-5);
		assert!(ProneAscent.effects_for(&end, 0.5).is_identity());
		assert_eq!(start.scratch.depth, 0);
		assert_eq!(end.scratch.depth, 0);
		Ok(())
	}

	#[test]
	fn push_is_finite_and_restores_endpoint_rotations() -> anyhow::Result<()> {
		let mid = sample(0.2);
		let no_push = ProneAscent::arm_push(&ProneAscent, 0.2);
		assert!(no_push > 0.0 && no_push.is_finite());
		for name in ["humerus.L", "humerus.R"] {
			assert!(mid.rotation(name).is_finite());
		}
		assert!(ProneAscent::arm_push(&ProneAscent, 0.0).abs() < 1e-6);
		assert!(ProneAscent::arm_push(&ProneAscent, 1.0).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn fine_samples_converge_and_repeated_samples_are_stateless() -> anyhow::Result<()> {
		let mut previous = sample(0.0);
		let mut step_60 = 0.0_f32;
		for frame in 1..=60 {
			let current = sample(frame as f32 / 60.0);
			step_60 = step_60.max(max_rotation_delta(&current, &previous));
			previous = current;
		}
		let mut previous = sample(0.0);
		let mut step_240 = 0.0_f32;
		for frame in 1..=240 {
			let current = sample(frame as f32 / 240.0);
			step_240 = step_240.max(max_rotation_delta(&current, &previous));
			previous = current;
		}
		assert!(step_60.is_finite() && step_60 > 0.02);
		assert!(step_240 < step_60 * 0.35, "240 Hz {step_240} vs 60 Hz {step_60}");
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneAscent.apply(&mut rig, 0.32);
		let first = rig.pose.clone();
		ProneAscent.apply(&mut rig, 0.87);
		ProneAscent.apply(&mut rig, 0.32);
		assert_eq!(rig.pose, first);
		assert_eq!(rig.scratch.depth, 0);
		let transition = Transition::from_visible(ProneAscent, first, ArmatureOffset::IDENTITY);
		let mut interrupted = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut interrupted, 0.32, 0.0);
		assert_eq!(interrupted.pose, rig.pose);
		Ok(())
	}
}
