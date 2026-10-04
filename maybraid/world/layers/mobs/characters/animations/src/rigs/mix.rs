use std::cell::RefCell;
use std::collections::HashSet;

use bevy::prelude::*;
use character_rigs::{humanoid::HumanoidRig, BonePose, RigPose};

use crate::animations::{Mix, Smooth};
use crate::{Animation, Effects};

/// Reused pose buffers for snapshot / sample / blend hot paths.
#[derive(Debug, Default)]
pub(crate) struct PoseScratch {
	rest: RefCell<RigPose>,
	from_pose: RefCell<RigPose>,
	to_pose: RefCell<RigPose>,
}

impl PoseScratch {
	pub fn rest_mut(&self) -> std::cell::RefMut<'_, RigPose> {
		self.rest.borrow_mut()
	}

	pub fn rest(&self) -> std::cell::Ref<'_, RigPose> {
		self.rest.borrow()
	}

	pub fn from_pose(&self) -> std::cell::Ref<'_, RigPose> {
		self.from_pose.borrow()
	}

	pub fn from_pose_mut(&self) -> std::cell::RefMut<'_, RigPose> {
		self.from_pose.borrow_mut()
	}

	pub fn to_pose(&self) -> std::cell::Ref<'_, RigPose> {
		self.to_pose.borrow()
	}

	pub fn to_pose_mut(&self) -> std::cell::RefMut<'_, RigPose> {
		self.to_pose.borrow_mut()
	}
}

thread_local! {
	pub(crate) static POSE_SCRATCH: PoseScratch = PoseScratch::default();
}

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
	POSE_SCRATCH.with(|scratch| {
		snapshot_pose_into(rig, &mut *scratch.rest_mut());
		sample_pose_into(from, rig, &*scratch.rest(), from_progress, &mut *scratch.from_pose_mut());
		sample_pose_into(to, rig, &*scratch.rest(), to_progress, &mut *scratch.to_pose_mut());
		blend_pose(rig, &*scratch.from_pose(), &*scratch.to_pose(), weight);
	});
}

pub(crate) fn snapshot_pose_into<R: HumanoidRig>(rig: &R, pose: &mut RigPose) {
	let mut present = HashSet::new();
	for bone in rig.animation_bones() {
		if let Some(source) = rig.pose().get(&bone) {
			present.insert(bone.clone());
			match pose.get_mut(&bone) {
				Some(existing) => existing.copy_fields_from(source),
				None => pose.upsert_from(source),
			}
		}
	}
	pose.retain_bones(&present);
}

pub(crate) fn snapshot_pose<R: HumanoidRig>(rig: &R) -> RigPose {
	let mut pose = RigPose::new();
	snapshot_pose_into(rig, &mut pose);
	pose
}

pub(crate) fn restore_pose<R: HumanoidRig>(rig: &mut R, rest: &RigPose) {
	for bone in rig.animation_bones() {
		if let Some(source) = rest.get(&bone) {
			rig.pose_mut().upsert_from(source);
		}
	}
}

#[allow(dead_code)]
pub(crate) fn sample<A: Animation<R>, R: HumanoidRig>(
	anim: &A,
	rig: &mut R,
	rest: &RigPose,
	progress: f32,
) -> (RigPose, Effects) {
	let mut pose = RigPose::new();
	let effects = sample_pose_into(anim, rig, rest, progress, &mut pose);
	(pose, effects)
}

fn sample_pose_into<A: Animation<R>, R: HumanoidRig>(
	anim: &A,
	rig: &mut R,
	rest: &RigPose,
	progress: f32,
	out: &mut RigPose,
) -> Effects {
	restore_pose(rig, rest);
	anim.apply_for(rig, progress);
	snapshot_pose_into(rig, out);
	anim.effects_for(rig, progress)
}

pub(crate) fn blend_pose<R: HumanoidRig>(rig: &mut R, from: &RigPose, to: &RigPose, weight: f32) {
	for bone in rig.animation_bones() {
		let Some(from_bone) = from.get(&bone) else {
			if let Some(to_bone) = to.get(&bone) {
				rig.pose_mut().upsert_from(to_bone);
			}
			continue;
		};
		let to_bone = to.get(&bone).unwrap_or(from_bone);
		match rig.pose_mut().get_mut(&bone) {
			Some(existing) => blend_bone_into(existing, from_bone, to_bone, weight),
			None => rig.pose_mut().insert(blend_bone(from_bone, to_bone, weight)),
		}
	}
}

fn blend_bone_into(existing: &mut BonePose, from: &BonePose, to: &BonePose, weight: f32) {
	let weight = weight.clamp(0.0, 1.0);
	existing.transform = Transform {
		translation: from.transform.translation.lerp(to.transform.translation, weight),
		rotation: from.transform.rotation.slerp(to.transform.rotation, weight),
		scale: from.transform.scale.lerp(to.transform.scale, weight),
	};
	existing.swing = from.swing + (to.swing - from.swing) * weight;
	existing.flex = from.flex + (to.flex - from.flex) * weight;
	existing.twist = from.twist + (to.twist - from.twist) * weight;
}

fn blend_bone(from: &BonePose, to: &BonePose, weight: f32) -> BonePose {
	let weight = weight.clamp(0.0, 1.0);
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
	POSE_SCRATCH.with(|scratch| {
		snapshot_pose_into(rig, &mut *scratch.rest_mut());
		sample_pose_into(anim, rig, &*scratch.rest(), progress, &mut *scratch.from_pose_mut());
		restore_pose(rig, &*scratch.rest());
		scratch.from_pose().clone()
	})
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
	use std::hint::black_box;
	use std::time::Instant;

	use character_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::{Mix, Spring, Squat};

	mod legacy {
		use bevy::prelude::Transform;
		use character_rigs::{humanoid::HumanoidRig, BonePose, RigPose};

		use crate::Animation;

		pub fn snapshot_pose<R: HumanoidRig>(rig: &R) -> RigPose {
			let mut pose = RigPose::new();
			for bone in rig.animation_bones() {
				if let Some(p) = rig.pose().get(&bone) {
					pose.insert(p.clone());
				}
			}
			pose
		}

		pub fn restore_pose<R: HumanoidRig>(rig: &mut R, rest: &RigPose) {
			for bone in rig.animation_bones() {
				if let Some(p) = rest.get(&bone) {
					rig.pose_mut().insert(p.clone());
				}
			}
		}

		pub fn blend_pose<R: HumanoidRig>(rig: &mut R, from: &RigPose, to: &RigPose, weight: f32) {
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

		pub fn sample_pose<A: Animation<R>, R: HumanoidRig>(
			anim: &A,
			rig: &mut R,
			rest: &RigPose,
			progress: f32,
		) -> RigPose {
			restore_pose(rig, rest);
			anim.apply_for(rig, progress);
			snapshot_pose(rig)
		}

		pub fn blend_poses<A, B, R>(
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
	}

	fn poses_equal(a: &RigPose, b: &RigPose) -> bool {
		if a.len() != b.len() {
			return false;
		}
		a.iter()
			.all(|(name, pose)| b.get(name).map(|other| pose == other).unwrap_or(false))
	}

	fn seeded_rig() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		rig
	}

	fn dirty_scratch() -> PoseScratch {
		let scratch = PoseScratch::default();
		scratch
			.rest_mut()
			.insert(BonePose::new("stale.A", Transform::from_translation(Vec3::X)));
		scratch
			.rest_mut()
			.insert(BonePose::new("stale.B", Transform::from_translation(Vec3::Y)));
		scratch
			.from_pose_mut()
			.insert(BonePose::new("stale.C", Transform::from_translation(Vec3::Z)));
		scratch.to_pose_mut().insert(BonePose::new("stale.D", Transform::IDENTITY));
		scratch
	}

	#[test]
	fn snapshot_pose_into_matches_allocating_path() {
		let rig = seeded_rig();
		let expected = legacy::snapshot_pose(&rig);
		let scratch = dirty_scratch();
		snapshot_pose_into(&rig, &mut *scratch.rest_mut());
		assert!(poses_equal(&expected, &*scratch.rest()));
		assert!(!scratch.rest().iter().any(|(name, _)| name.as_str().starts_with("stale.")));
	}

	#[test]
	fn snapshot_pose_into_reuses_capacity_on_repeat() {
		let rig = seeded_rig();
		let scratch = dirty_scratch();
		snapshot_pose_into(&rig, &mut *scratch.rest_mut());
		let capacity_after_first = scratch.rest().capacity();
		snapshot_pose_into(&rig, &mut *scratch.rest_mut());
		assert!(scratch.rest().capacity() >= capacity_after_first);
		assert!(poses_equal(&legacy::snapshot_pose(&rig), &*scratch.rest()));
	}

	#[test]
	fn restore_pose_matches_allocating_path() {
		let mut rig_a = seeded_rig();
		let mut rig_b = seeded_rig();
		Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0).apply(&mut rig_a, 0.25);
		let rest = legacy::snapshot_pose(&rig_a);
		legacy::restore_pose(&mut rig_a, &rest);
		restore_pose(&mut rig_b, &rest);
		assert!(poses_equal(rig_a.pose(), rig_b.pose()));
	}

	#[test]
	fn blend_pose_matches_allocating_path() {
		let mut rig_a = seeded_rig();
		let mut rig_b = seeded_rig();
		let from = legacy::snapshot_pose(&rig_a);
		Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0).apply(&mut rig_a, 0.5);
		let to = legacy::snapshot_pose(&rig_a);
		legacy::blend_pose(&mut rig_a, &from, &to, 0.35);
		blend_pose(&mut rig_b, &from, &to, 0.35);
		assert!(poses_equal(rig_a.pose(), rig_b.pose()));
	}

	#[test]
	fn sample_pose_into_matches_allocating_path_with_dirty_scratch() {
		let mut rig_a = seeded_rig();
		let mut rig_b = seeded_rig();
		let rest = legacy::snapshot_pose(&rig_a);
		let anim = Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0);
		let expected = legacy::sample_pose(&anim, &mut rig_a, &rest, 0.35);
		let scratch = dirty_scratch();
		sample_pose_into(&anim, &mut rig_b, &rest, 0.35, &mut *scratch.from_pose_mut());
		assert!(poses_equal(&expected, &*scratch.from_pose()));
		assert!(!scratch.from_pose().iter().any(|(name, _)| name.as_str().starts_with("stale.")));
	}

	#[test]
	fn blend_poses_matches_allocating_path_with_dirty_scratch() {
		let mut rig_a = seeded_rig();
		let mut rig_b = seeded_rig();
		let mix = Mix::<_, _, HumanoidV0Rig>::new(
			Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0),
			Spring::<HumanoidV0Rig>::default(),
			0.5,
		);
		legacy::blend_poses(&mut rig_a, &mix.from, &mix.to, 0.25, 0.75, mix.weight);
		let scratch = dirty_scratch();
		snapshot_pose_into(&rig_b, &mut *scratch.rest_mut());
		sample_pose_into(
			&mix.from,
			&mut rig_b,
			&*scratch.rest(),
			0.25,
			&mut *scratch.from_pose_mut(),
		);
		sample_pose_into(&mix.to, &mut rig_b, &*scratch.rest(), 0.75, &mut *scratch.to_pose_mut());
		blend_pose(&mut rig_b, &*scratch.from_pose(), &*scratch.to_pose(), mix.weight);
		assert!(poses_equal(rig_a.pose(), rig_b.pose()));
	}

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

	fn median(mut samples: Vec<f64>) -> f64 {
		samples.sort_unstable_by(|a, b| a.total_cmp(b));
		samples[samples.len() / 2]
	}

	#[test]
	#[ignore = "microbench: run with `cargo test -p character-animations snapshot_pose_scratch_bench -- --ignored --release`"]
	fn snapshot_pose_scratch_bench() {
		let rig = seeded_rig();
		let mix = Mix::<_, _, HumanoidV0Rig>::new(
			Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0),
			Spring::<HumanoidV0Rig>::default(),
			0.5,
		);
		let iterations = 2_000usize;
		let scratch = PoseScratch::default();

		// Warm scratch buffers to steady-state capacity.
		for _ in 0..32 {
			let mut warm = seeded_rig();
			blend_poses(&mut warm, &mix.from, &mix.to, 0.25, 0.75, mix.weight);
		}
		snapshot_pose_into(&rig, &mut *scratch.rest_mut());
		let steady_capacity = scratch.rest().capacity();

		let mut allocating = Vec::with_capacity(9);
		for _ in 0..9 {
			let start = Instant::now();
			for _ in 0..iterations {
				let mut local = seeded_rig();
				legacy::blend_poses(
					&mut local,
					&mix.from,
					&mix.to,
					black_box(0.25),
					black_box(0.75),
					black_box(mix.weight),
				);
			}
			allocating.push(start.elapsed().as_secs_f64() * 1_000.0);
		}

		let mut reused = Vec::with_capacity(9);
		for _ in 0..9 {
			let start = Instant::now();
			for _ in 0..iterations {
				let mut local = seeded_rig();
				blend_poses(
					&mut local,
					&mix.from,
					&mix.to,
					black_box(0.25),
					black_box(0.75),
					black_box(mix.weight),
				);
			}
			reused.push(start.elapsed().as_secs_f64() * 1_000.0);
		}

		eprintln!(
			"snapshot_pose_scratch_bench: character=HumanoidV0Rig, animation=Mix(Squat,Spring), method=blend_poses, iterations={iterations}"
		);
		eprintln!(
			"  allocating: min={:.2}ms median={:.2}ms",
			allocating.iter().copied().fold(f64::INFINITY, f64::min),
			median(allocating)
		);
		eprintln!(
			"  scratch reuse: min={:.2}ms median={:.2}ms, steady rest capacity={}",
			reused.iter().copied().fold(f64::INFINITY, f64::min),
			median(reused),
			steady_capacity
		);
	}
}
