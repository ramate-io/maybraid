use bevy::prelude::Quat;
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Idle, Squat, SquatAscent};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for SquatAscent {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		blend_clips(rig, &Squat::held(), 1.0, &Idle::default(), 0.0, 1.0 - self.depth(progress));
		let lead = self.shoulder_lead(progress);
		if lead > 0.0 {
			for side in [Side::Left, Side::Right] {
				let name = match side {
					Side::Left => "shoulder.L",
					Side::Right => "shoulder.R",
				};
				let Some(id) = rig.binding.definition.id(name) else {
					continue;
				};
				let current = rig.pose.rotation(id);
				rig.pose.set_rotation(
					id,
					Quat::from_rotation_z(self.shoulder_flex(side, progress)) * current,
				);
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
	use character_rigs::Side;

	use super::*;
	use crate::animations::Transition;

	fn max_rotation_delta(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).normalize().angle_between(b.rotation(name).normalize()))
			.fold(0.0, f32::max)
	}

	fn sample(progress: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatAscent.apply(&mut rig, progress);
		rig
	}

	#[test]
	fn endpoints_match_held_squat_and_idle_zero() -> anyhow::Result<()> {
		let start = sample(0.0);
		let end = sample(1.0);
		let mut squat = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut squat, 1.0);
		let mut idle = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut idle, 0.0);
		assert!(max_rotation_delta(&start, &squat) < 1e-5);
		assert!(max_rotation_delta(&end, &idle) < 1e-5);
		assert!(SquatAscent.effects_for(&end, 0.5).is_identity());
		assert_eq!(start.scratch.depth, 0);
		assert_eq!(end.scratch.depth, 0);
		Ok(())
	}

	#[test]
	fn shoulder_lead_is_sided_and_clears_at_held_and_idle_endpoints() -> anyhow::Result<()> {
		let mid = sample(0.15);
		let left = mid.character_point("shoulder.L");
		let right = mid.character_point("shoulder.R");
		let rest = HumanoidV0Rig::for_clip_test();
		let left_rise = left.y - rest.character_point("shoulder.L").y;
		let right_rise = right.y - rest.character_point("shoulder.R").y;
		assert!(left_rise * right_rise > 0.0, "both shoulders rise: {left:?} {right:?}");
		assert!((left_rise - right_rise).abs() < 0.03);
		for progress in [0.0, 1.0] {
			let endpoint = sample(progress);
			let reference = if progress == 0.0 { &sample_held() } else { &sample_idle() };
			assert!(max_rotation_delta(&endpoint, reference) < 1e-5);
		}
		assert!(
			SquatAscent.shoulder_flex(Side::Left, 0.15)
				* SquatAscent.shoulder_flex(Side::Right, 0.15)
				< 0.0
		);
		Ok(())
	}

	fn sample_held() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut rig, 1.0);
		rig
	}

	fn sample_idle() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut rig, 0.0);
		rig
	}

	#[test]
	fn motion_is_finite_smooth_and_does_not_accumulate() -> anyhow::Result<()> {
		let mut previous = sample(0.0);
		let mut max_60 = 0.0_f32;
		for frame in 1..=60 {
			let current = sample(frame as f32 / 60.0);
			max_60 = max_60.max(max_rotation_delta(&current, &previous));
			assert!(current.character_point("root").is_finite());
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
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatAscent.apply(&mut rig, 0.36);
		let first = rig.pose.clone();
		SquatAscent.apply(&mut rig, 0.9);
		SquatAscent.apply(&mut rig, 0.36);
		assert_eq!(rig.pose, first);
		assert_eq!(rig.scratch.depth, 0);
		let transition = Transition::from_visible(SquatAscent, first, ArmatureOffset::IDENTITY);
		let mut interrupted = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut interrupted, 0.36, 0.0);
		assert_eq!(interrupted.pose, rig.pose);
		Ok(())
	}
}
