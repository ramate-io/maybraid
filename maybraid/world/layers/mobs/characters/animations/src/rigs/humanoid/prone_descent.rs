use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{Idle, Prone, ProneDescent};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for ProneDescent {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		blend_clips(rig, &Idle::default(), 0.0, &Prone::default(), 1.0, self.depth(progress));
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		ArmatureOffset::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
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
		ProneDescent.apply(&mut rig, progress);
		rig
	}

	#[test]
	fn endpoints_match_idle_zero_and_held_prone() -> anyhow::Result<()> {
		let start = sample(0.0);
		let end = sample(1.0);
		let mut idle = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut idle, 0.0);
		let mut prone = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut prone, 1.0);
		assert!(max_rotation_delta(&start, &idle) < 1e-5);
		assert!(max_rotation_delta(&end, &prone) < 1e-5);
		assert!(ProneDescent.effects_for(&HumanoidV0Rig::for_clip_test(), 0.5).is_identity());
		assert_eq!(start.scratch.depth, 0);
		assert_eq!(end.scratch.depth, 0);
		Ok(())
	}

	#[test]
	fn fixed_endpoints_resample_smoothly_without_moving_root() -> anyhow::Result<()> {
		let root = Vec3::Y;
		let mut previous = sample(0.0);
		let mut max_60 = 0.0_f32;
		for frame in 1..=60 {
			let current = sample(frame as f32 / 60.0);
			max_60 = max_60.max(max_rotation_delta(&current, &previous));
			assert!(
				(current.rotation("root") * root - previous.rotation("root") * root).length()
					< 0.08
			);
			previous = current;
		}
		let mut previous = sample(0.0);
		let mut max_240 = 0.0_f32;
		for frame in 1..=240 {
			let current = sample(frame as f32 / 240.0);
			max_240 = max_240.max(max_rotation_delta(&current, &previous));
			previous = current;
		}
		assert!(max_60.is_finite() && max_60 > 0.02);
		assert!(max_240 < max_60 * 0.35, "240 Hz {max_240} vs 60 Hz {max_60}");
		let mut resampled = HumanoidV0Rig::for_clip_test();
		ProneDescent.apply(&mut resampled, 0.73);
		let captured = resampled.pose.clone();
		ProneDescent.apply(&mut resampled, 0.11);
		ProneDescent.apply(&mut resampled, 0.73);
		assert_eq!(resampled.pose, sample(0.73).pose, "out-of-order sampling is stateless");
		assert_eq!(resampled.pose, captured);
		assert_eq!(resampled.scratch.depth, 0);
		let transition = Transition::from_visible(
			ProneDescent,
			resampled.pose.clone(),
			ArmatureOffset::IDENTITY,
		);
		let captured = resampled.pose.clone();
		transition.apply(&mut resampled, 0.5, 0.0);
		assert_eq!(resampled.pose, captured);
		Ok(())
	}
}
