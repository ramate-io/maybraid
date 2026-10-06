use character_rigs::authoring::PoseBuffer;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::Transition;
use crate::rigs::mix::{capture_into, mix_effects};
use crate::{Animation, Effects};

impl<A> Transition<A>
where
	A: Animation<HumanoidV0Rig>,
{
	/// Creates a transition into `animation`, capturing the rig's current pose and
	/// treating the current armature offset as identity unless the caller stored one.
	pub fn new(animation: A, rig: &HumanoidV0Rig) -> Self {
		Self::from_pose(animation, rig.pose.clone())
	}

	pub fn apply(
		&self,
		rig: &mut HumanoidV0Rig,
		animation_progress: f32,
		transition_progress: f32,
	) -> Effects {
		self.animation.apply_for(rig, animation_progress);
		let effects = self.animation.effects_for(rig, animation_progress);
		rig.scratch.b.copy_from(&rig.pose);
		let weight = self.weight(transition_progress);
		PoseBuffer::blend_into(&self.from_pose, &rig.scratch.b, weight, &mut rig.pose);
		mix_effects(self.from_offset, effects, weight)
	}
}

/// Samples an animation into an owned buffer. Prefer [`capture_into`] on a reused slot.
pub fn capture_animation_pose<A>(anim: &A, rig: &mut HumanoidV0Rig, progress: f32) -> PoseBuffer
where
	A: Animation<HumanoidV0Rig>,
{
	let mut dest = PoseBuffer::identity(rig.pose.len());
	capture_into(anim, rig, progress, &mut dest);
	dest
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::*;

	use crate::animations::{Fall, Land, Spring, Squat, Transition, TransitionCurve};
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	#[test]
	fn transition_at_zero_matches_from_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Fall::default().apply(&mut rig, 1.0);
		let from_pose = rig.pose.clone();

		let mut out = HumanoidV0Rig::imported();
		Transition::from_pose(Squat::for_loop(1.0, 1.0), from_pose).apply(&mut out, 0.5, 0.0);

		assert!(rig.rotation("femur.L").dot(out.rotation("femur.L")).abs() > 1.0 - 1e-5);
		Ok(())
	}

	#[test]
	fn transition_at_one_matches_target_animation() -> anyhow::Result<()> {
		let rig = HumanoidV0Rig::imported();
		let from_pose = rig.pose.clone();
		let target = Squat::for_loop(1.0, 1.0);

		let mut expected = HumanoidV0Rig::imported();
		target.apply(&mut expected, 0.5);

		let mut out = HumanoidV0Rig::imported();
		Transition::from_pose(target, from_pose).apply(&mut out, 0.5, 1.0);

		assert!(expected.rotation("femur.L").dot(out.rotation("femur.L")).abs() > 1.0 - 1e-5);
		Ok(())
	}

	#[test]
	fn fall_to_land_transition_blends_arms() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let from_pose = capture_animation_pose(&Fall::default(), &mut rig, 1.0);
		let land = Land::default();
		let land_progress = 0.05 / land.cycle_duration();

		Transition::from_pose(land, from_pose)
			.with_curve(TransitionCurve::SmoothStep)
			.apply(&mut rig, land_progress, 0.5);

		assert!(rig.posed_angle("shoulder.L") > 0.02, "blended shoulder leaves rest");
		Ok(())
	}

	#[test]
	fn spring_transition_from_stand() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let from_pose = capture_animation_pose(&Squat::for_loop(1.0, 1.0), &mut rig, 0.0);

		Transition::from_pose(Spring::default(), from_pose).apply(&mut rig, 0.5, 0.5);

		assert!(rig.posed_angle("femur.L") > 0.0);
		let mut full = HumanoidV0Rig::imported();
		Squat::for_loop(1.0, 1.0).apply(&mut full, 0.5);
		assert!(rig.posed_angle("femur.L") < full.posed_angle("femur.L"));
		Ok(())
	}

	#[test]
	fn interrupted_transition_starts_from_the_visible_offset() {
		let mut rig = HumanoidV0Rig::imported();
		let from_pose = rig.pose.clone();
		let from_offset = ArmatureOffset::from_translation(Vec3::new(0.0, -0.2, 0.0));
		let effects = Transition::from_visible(Squat::for_loop(1.0, 1.0), from_pose, from_offset)
			.apply(&mut rig, 0.0, 0.0);
		assert!((effects.0.translation - from_offset.0.translation).length() < 1e-5);
	}
}
