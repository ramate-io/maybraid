use crozon_rigs::{humanoid::HumanoidRig, RigPose};

use crate::animations::Transition;
use crate::rigs::mix::{blend_pose, mix_effects, pose_from_animation, sample, snapshot_pose};
use crate::{Effects, Animation};

impl<A, R> Transition<A, R>
where
	R: HumanoidRig,
{
	/// Creates a transition into `animation`, capturing the rig's current pose.
	pub fn new(animation: A, rig: &R, progress: f32) -> Self {
		Self::from_pose(animation, snapshot_pose(rig), progress)
	}
}

impl<A, R> Animation<R> for Transition<A, R>
where
	A: Animation<R>,
	R: HumanoidRig,
{
	fn apply(&self, rig: &mut R) -> Effects {
		let rest = snapshot_pose(rig);
		let (target_pose, target_effects) = sample(&self.animation, rig, &rest);
		let weight = self.weight();
		blend_pose(rig, &self.from_pose, &target_pose, weight);
		mix_effects(Effects::default(), target_effects, weight)
	}
}

/// Samples an animation into a pose without leaving the rig in that state.
pub fn capture_animation_pose<A, R>(anim: &A, rig: &mut R) -> RigPose
where
	A: Animation<R>,
	R: HumanoidRig,
{
	pose_from_animation(anim, rig)
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::{BlendCurve, Fall, Spring, Squat, Transition};
	use crate::rigs::mix::seed_bind_pose;

	#[test]
	fn transition_at_zero_matches_from_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::spread().apply(&mut rig);
		let from_pose = snapshot_pose(&rig);

		let mut out = HumanoidV0Rig::imported();
		seed_bind_pose(&mut out);
		Transition::from_pose(Squat::<HumanoidV0Rig>::new(0.5), from_pose, 0.0).apply(&mut out);

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
		let target = Squat::<HumanoidV0Rig>::new(0.5);

		let mut expected = HumanoidV0Rig::imported();
		seed_bind_pose(&mut expected);
		target.clone().apply(&mut expected);

		let mut out = HumanoidV0Rig::imported();
		seed_bind_pose(&mut out);
		Transition::from_pose(target, from_pose, 1.0).apply(&mut out);

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
		let from_pose = capture_animation_pose(&Fall::<HumanoidV0Rig>::spread(), &mut rig);
		let land = crate::animations::Land::<HumanoidV0Rig>::default().at_segment_time(0.05);

		Transition::from_pose(land, from_pose, 0.5)
			.with_curve(BlendCurve::SmoothStep)
			.apply(&mut rig);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		assert!(shoulder.flex.abs() > 0.05);
		Ok(())
	}

	#[test]
	fn spring_transition_from_stand() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let from_pose = capture_animation_pose(&Squat::<HumanoidV0Rig>::new(0.0), &mut rig);

		Transition::from_pose(Spring::<HumanoidV0Rig>::at_extension(0.5), from_pose, 0.5)
			.apply(&mut rig);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		assert!(femur.swing.abs() > 0.0);
		assert!(femur.swing.abs() < Squat::<HumanoidV0Rig>::new(0.5).femur_swing().abs());
		Ok(())
	}
}
