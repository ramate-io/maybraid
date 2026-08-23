use bevy::prelude::*;
use crozon_rigs::{humanoid::HumanoidRig, BonePose, RigPose};

use crate::animations::{Mix, Smooth};
use crate::{Animation, Effects};

impl<A, B, R> Animation<R> for Mix<A, B, R>
where
	A: Animation<R>,
	B: Animation<R>,
	R: HumanoidRig,
{
	fn apply_for(&self, rig: &mut R, progress: f32) {
		blend_poses(rig, &self.from, &self.to, progress, progress, self.weight);
	}

	fn effects_for(&self, rig: &R, progress: f32) -> Effects {
		mix_effects(
			self.from.effects_for(rig, progress),
			self.to.effects_for(rig, progress),
			self.weight,
		)
	}
}

impl<A, B, R> Mix<A, B, R>
where
	A: Animation<R>,
	B: Animation<R>,
	R: HumanoidRig,
{
	pub fn apply_at(&self, rig: &mut R, from_progress: f32, to_progress: f32) -> Effects {
		blend_animations(rig, &self.from, &self.to, from_progress, to_progress, self.weight)
	}
}

impl<A, B, R> Animation<R> for Smooth<A, B, R>
where
	A: Animation<R>,
	B: Animation<R>,
	R: HumanoidRig,
{
	fn apply_for(&self, rig: &mut R, progress: f32) {
		blend_poses(
			rig,
			&self.from,
			&self.to,
			progress,
			progress,
			crate::animations::smoothstep(self.weight),
		);
	}

	fn effects_for(&self, rig: &R, progress: f32) -> Effects {
		mix_effects(
			self.from.effects_for(rig, progress),
			self.to.effects_for(rig, progress),
			crate::animations::smoothstep(self.weight),
		)
	}
}

impl<A, B, R> Smooth<A, B, R>
where
	A: Animation<R>,
	B: Animation<R>,
	R: HumanoidRig,
{
	pub fn apply_at(&self, rig: &mut R, from_progress: f32, to_progress: f32) -> Effects {
		blend_animations(
			rig,
			&self.from,
			&self.to,
			from_progress,
			to_progress,
			crate::animations::smoothstep(self.weight),
		)
	}
}

fn blend_animations<A, B, R>(
	rig: &mut R,
	from: &A,
	to: &B,
	from_progress: f32,
	to_progress: f32,
	weight: f32,
) -> Effects
where
	A: Animation<R>,
	B: Animation<R>,
	R: HumanoidRig,
{
	blend_poses(rig, from, to, from_progress, to_progress, weight);
	mix_effects(from.effects_for(rig, from_progress), to.effects_for(rig, to_progress), weight)
}

fn blend_poses<A, B, R>(
	rig: &mut R,
	from: &A,
	to: &B,
	from_progress: f32,
	to_progress: f32,
	weight: f32,
) where
	A: Animation<R>,
	B: Animation<R>,
	R: HumanoidRig,
{
	let rest = snapshot_pose(rig);
	let from_pose = sample_pose(from, rig, &rest, from_progress);
	let to_pose = sample_pose(to, rig, &rest, to_progress);
	blend_pose(rig, &from_pose, &to_pose, weight);
}

pub(crate) fn snapshot_pose<R: HumanoidRig>(rig: &R) -> RigPose {
	let mut pose = RigPose::new();
	for bone in rig.animation_bones() {
		if let Some(p) = rig.pose().get(&bone) {
			pose.insert(p.clone());
		}
	}
	pose
}

pub(crate) fn restore_pose<R: HumanoidRig>(rig: &mut R, rest: &RigPose) {
	for bone in rig.animation_bones() {
		if let Some(p) = rest.get(&bone) {
			rig.pose_mut().insert(p.clone());
		}
	}
}

pub(crate) fn sample<A: Animation<R>, R: HumanoidRig>(
	anim: &A,
	rig: &mut R,
	rest: &RigPose,
	progress: f32,
) -> (RigPose, Effects) {
	let pose = sample_pose(anim, rig, rest, progress);
	(pose, anim.effects_for(rig, progress))
}

fn sample_pose<A: Animation<R>, R: HumanoidRig>(
	anim: &A,
	rig: &mut R,
	rest: &RigPose,
	progress: f32,
) -> RigPose {
	restore_pose(rig, rest);
	anim.apply_for(rig, progress);
	snapshot_pose(rig)
}

pub(crate) fn blend_pose<R: HumanoidRig>(rig: &mut R, from: &RigPose, to: &RigPose, weight: f32) {
	for bone in rig.animation_bones() {
		let Some(from_bone) = from.get(&bone) else {
			if let Some(to_bone) = to.get(&bone) {
				rig.pose_mut().insert(to_bone.clone());
			}
			continue;
		};
		let to_bone = to.get(&bone).unwrap_or(from_bone);
		rig.pose_mut().insert(blend_bone(from_bone, to_bone, weight));
	}
}

fn blend_bone(from: &BonePose, to: &BonePose, weight: f32) -> BonePose {
	BonePose {
		name: from.name.clone(),
		transform: Transform {
			translation: from.transform.translation.lerp(to.transform.translation, weight),
			rotation: from.transform.rotation.slerp(to.transform.rotation, weight),
			scale: from.transform.scale.lerp(to.transform.scale, weight),
		},
		swing: from.swing + (to.swing - from.swing) * weight,
		flex: from.flex + (to.flex - from.flex) * weight,
		twist: from.twist + (to.twist - from.twist) * weight,
	}
}

pub(crate) fn mix_effects(from: Effects, to: Effects, weight: f32) -> Effects {
	match (from.r#move, to.r#move) {
		(None, None) => Effects::default(),
		(Some(m), None) => Effects { r#move: Some(scale_transform(m, 1.0 - weight)) },
		(None, Some(m)) => Effects { r#move: Some(scale_transform(m, weight)) },
		(Some(a), Some(b)) => Effects { r#move: Some(lerp_transform(a, b, weight)) },
	}
}

fn lerp_transform(a: Transform, b: Transform, t: f32) -> Transform {
	Transform {
		translation: a.translation.lerp(b.translation, t),
		rotation: a.rotation.slerp(b.rotation, t),
		scale: a.scale.lerp(b.scale, t),
	}
}

fn scale_transform(t: Transform, scale: f32) -> Transform {
	Transform { translation: t.translation * scale, rotation: t.rotation, scale: t.scale }
}

pub(crate) fn pose_from_animation<A: Animation<R>, R: HumanoidRig>(
	anim: &A,
	rig: &mut R,
	progress: f32,
) -> RigPose {
	let rest = snapshot_pose(rig);
	let (pose, _effects) = sample(anim, rig, &rest, progress);
	restore_pose(rig, &rest);
	pose
}

#[allow(dead_code)]
pub(crate) fn seed_bind_pose(rig: &mut impl HumanoidRig) {
	for bone in rig.animation_bones() {
		if rig.pose().get(&bone).is_none() {
			rig.pose_mut().insert(BonePose::new(bone, Transform::IDENTITY));
		}
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::{Mix, Spring, Squat};

	#[test]
	fn mix_interpolates_femur_swing() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);

		let mix = Mix::<_, _, HumanoidV0Rig>::new(
			Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0),
			Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0),
			0.5,
		);
		mix.apply_at(&mut rig, 0.0, 0.5);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		let full = Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0).femur_swing(0.5);
		assert!(femur.swing.abs() > 0.0);
		assert!(femur.swing.abs() < full.abs());
		Ok(())
	}

	#[test]
	fn mix_at_zero_matches_from() -> anyhow::Result<()> {
		let mut rig_a = HumanoidV0Rig::imported();
		let mut rig_b = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig_a);
		seed_bind_pose(&mut rig_b);

		Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0).apply(&mut rig_a, 0.25);
		Mix::<_, _, HumanoidV0Rig>::new(
			Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0),
			Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0),
			0.0,
		)
		.apply_at(&mut rig_b, 0.25, 0.75);

		let bone = rig_a.leg(Side::Left).femur.name.clone();
		assert_eq!(
			rig_a.pose().get(&bone).expect("a").swing,
			rig_b.pose().get(&bone).expect("b").swing
		);
		Ok(())
	}

	#[test]
	fn smooth_spring_from_stand_blends_arms() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);

		Smooth::<_, _, HumanoidV0Rig>::new(
			Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0),
			Spring::<HumanoidV0Rig>::default(),
			0.5,
		)
		.apply_at(&mut rig, 0.0, 1.0);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		assert!(shoulder.swing < 0.0);
		Ok(())
	}
}
