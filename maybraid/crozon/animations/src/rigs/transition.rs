use crozon_rigs::{humanoid::HumanoidRig, RigPose};

use crate::animations::Transition;
use crate::rigs::mix::{blend_pose, mix_effects, pose_from_animation, restore_pose, snapshot_pose};
use crate::{Animation, Effects};

impl<A, R> Transition<A, R>
where
	A: Animation<R>,
	R: HumanoidRig,
{
	/// Creates a transition into `animation`, capturing the rig's current pose.
	pub fn new(animation: A, rig: &R) -> Self {
		Self::from_pose(animation, snapshot_pose(rig))
	}

	pub fn apply(&self, rig: &mut R, animation_progress: f32, transition_progress: f32) -> Effects {
		let rest = snapshot_pose(rig);
		restore_pose(rig, &rest);
		self.animation.apply_for(rig, animation_progress);
		let effects = self.animation.effects_for(rig, animation_progress);
		let target_pose = snapshot_pose(rig);
		let weight = self.weight(transition_progress);
		blend_pose(rig, &self.from_pose, &target_pose, weight);
		mix_effects(Effects::default(), effects, weight)
	}
}

/// Samples an animation into a pose without leaving the rig in that state.
pub fn capture_animation_pose<A, R>(anim: &A, rig: &mut R, progress: f32) -> RigPose
where
	A: Animation<R>,
	R: HumanoidRig,
{
	pose_from_animation(anim, rig, progress)
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::{Fall, Land, Spring, Squat, Transition, TransitionCurve};
	use crate::rigs::mix::seed_bind_pose;

	#[test]
	fn transition_at_zero_matches_from_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut rig, 1.0);
		let from_pose = snapshot_pose(&rig);

		let mut out = HumanoidV0Rig::imported();
		seed_bind_pose(&mut out);
		Transition::from_pose(Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0), from_pose)
			.apply(&mut out, 0.5, 0.0);

		let bone = rig.leg(Side::Left).femur.name.clone();
		assert_eq!(
			rig.pose().get(&bone).expect("from").swing,
			out.pose().get(&bone).expect("out").swing
		);
		Ok(())
	}

	#[test]
	fn transition_at_one_matches_target_animation() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let from_pose = snapshot_pose(&rig);
		let target = Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0);

		let mut expected = HumanoidV0Rig::imported();
		seed_bind_pose(&mut expected);
		target.apply(&mut expected, 0.5);

		let mut out = HumanoidV0Rig::imported();
		seed_bind_pose(&mut out);
		Transition::from_pose(target, from_pose).apply(&mut out, 0.5, 1.0);

		let bone = rig.leg(Side::Left).femur.name.clone();
		assert_eq!(
			expected.pose().get(&bone).expect("expected").swing,
			out.pose().get(&bone).expect("out").swing
		);
		Ok(())
	}

	#[test]
	fn fall_to_land_transition_blends_arms() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let from_pose = capture_animation_pose(&Fall::<HumanoidV0Rig>::default(), &mut rig, 1.0);
		let land = Land::<HumanoidV0Rig>::default();
		let land_progress = 0.05 / land.cycle_duration();

		Transition::from_pose(land, from_pose)
			.with_curve(TransitionCurve::SmoothStep)
			.apply(&mut rig, land_progress, 0.5);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		assert!(shoulder.flex.abs() > 0.05);
		Ok(())
	}

	#[test]
	fn spring_transition_from_stand() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let from_pose =
			capture_animation_pose(&Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0), &mut rig, 0.0);

		Transition::from_pose(Spring::<HumanoidV0Rig>::default(), from_pose)
			.apply(&mut rig, 0.5, 0.5);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		assert!(femur.swing.abs() > 0.0);
		assert!(
			femur.swing.abs() < Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0).femur_swing(0.5).abs()
		);
		Ok(())
	}
}
