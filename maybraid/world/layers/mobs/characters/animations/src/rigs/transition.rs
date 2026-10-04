use character_rigs::{humanoid::HumanoidRig, RigPose};

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
	use std::hint::black_box;
	use std::time::Instant;

	use character_rigs::{rigs::humanoid_v0::HumanoidV0Rig, RigPose, Side};

	use super::*;
	use crate::animations::{Fall, Land, Spring, Squat, Transition, TransitionCurve};
	use crate::rigs::mix::seed_bind_pose;
	use crate::Effects;

	/// Pre-optimization reference implementation (includes the redundant snapshot/restore).
	fn apply_legacy<A, R>(
		transition: &Transition<A, R>,
		rig: &mut R,
		animation_progress: f32,
		transition_progress: f32,
	) -> Effects
	where
		A: Animation<R>,
		R: HumanoidRig,
	{
		let rest = snapshot_pose(rig);
		restore_pose(rig, &rest);
		transition.animation.apply_for(rig, animation_progress);
		let effects = transition.animation.effects_for(rig, animation_progress);
		let target_pose = snapshot_pose(rig);
		let weight = transition.weight(transition_progress);
		blend_pose(rig, &transition.from_pose, &target_pose, weight);
		mix_effects(Effects::default(), effects, weight)
	}

	fn clone_rig(rig: &HumanoidV0Rig) -> HumanoidV0Rig {
		let mut out = HumanoidV0Rig::imported();
		for bone in rig.animation_bones() {
			if let Some(pose) = rig.pose().get(&bone) {
				out.pose_mut().insert(pose.clone());
			}
		}
		out
	}

	fn assert_poses_match(expected: &HumanoidV0Rig, actual: &HumanoidV0Rig) {
		for bone in expected.animation_bones() {
			match (expected.pose().get(&bone), actual.pose().get(&bone)) {
				(Some(a), Some(b)) => {
					assert_eq!(a.swing, b.swing, "bone {bone:?} swing");
					assert_eq!(a.flex, b.flex, "bone {bone:?} flex");
					assert_eq!(a.twist, b.twist, "bone {bone:?} twist");
					assert_eq!(
						a.transform.translation, b.transform.translation,
						"bone {bone:?} translation"
					);
				}
				(None, None) => {}
				(a, b) => panic!("bone {bone:?} presence mismatch: {a:?} vs {b:?}"),
			}
		}
	}

	fn assert_effects_match(expected: Effects, actual: Effects) {
		match (expected.r#move, actual.r#move) {
			(None, None) => {}
			(Some(a), Some(b)) => {
				assert!(
					(a.translation - b.translation).length() < 1e-5,
					"move translation mismatch: {:?} vs {:?}",
					a.translation,
					b.translation
				);
				assert!(
					a.rotation.angle_between(b.rotation).abs() < 1e-5,
					"move rotation mismatch"
				);
			}
			(a, b) => panic!("Effects.move mismatch: {a:?} vs {b:?}"),
		}
	}

	fn partial_from_pose(rig: &HumanoidV0Rig, bone_names: &[&str]) -> RigPose {
		let mut from_pose = RigPose::new();
		for name in bone_names {
			let name = (*name).into();
			if let Some(pose) = rig.pose().get(&name) {
				from_pose.insert(pose.clone());
			}
		}
		from_pose
	}

	fn assert_matches_legacy_at_weight(
		from_pose: RigPose,
		animation_progress: f32,
		transition_progress: f32,
	) {
		let target = Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0);
		let transition = Transition::from_pose(target.clone(), from_pose);

		let mut legacy_rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut legacy_rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut legacy_rig, 0.25);
		let mut optimized_rig = clone_rig(&legacy_rig);

		let legacy_effects =
			apply_legacy(&transition, &mut legacy_rig, animation_progress, transition_progress);
		let optimized_effects =
			transition.apply(&mut optimized_rig, animation_progress, transition_progress);

		assert_poses_match(&legacy_rig, &optimized_rig);
		assert_effects_match(legacy_effects, optimized_effects);
	}

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
	fn transition_at_zero_matches_legacy_full_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut rig, 1.0);
		let from_pose = snapshot_pose(&rig);
		assert_matches_legacy_at_weight(from_pose, 0.5, 0.0);
	}

	#[test]
	fn transition_at_one_matches_legacy_full_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let from_pose = snapshot_pose(&rig);
		assert_matches_legacy_at_weight(from_pose, 0.5, 1.0);
	}

	#[test]
	fn transition_at_zero_matches_legacy_partial_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut rig, 1.0);
		let femur = rig.leg(Side::Left).femur.name.to_string();
		let from_pose = partial_from_pose(&rig, &[femur.as_str()]);
		assert_matches_legacy_at_weight(from_pose, 0.5, 0.0);
	}

	#[test]
	fn transition_at_one_matches_legacy_partial_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut rig, 0.5);
		let femur = rig.leg(Side::Left).femur.name.to_string();
		let shoulder = rig.arm(Side::Left).shoulder.name.to_string();
		let from_pose = partial_from_pose(&rig, &[femur.as_str(), shoulder.as_str()]);
		assert_matches_legacy_at_weight(from_pose, 0.5, 1.0);
	}

	#[test]
	fn transition_at_zero_returns_legacy_effects_with_partial_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let femur = rig.leg(Side::Left).femur.name.to_string();
		let from_pose = partial_from_pose(&rig, &[femur.as_str()]);
		let transition =
			Transition::from_pose(Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0), from_pose);

		let mut legacy_rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut legacy_rig);
		let legacy_effects = apply_legacy(&transition, &mut legacy_rig, 0.5, 0.0);

		let mut optimized_rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut optimized_rig);
		let optimized_effects = transition.apply(&mut optimized_rig, 0.5, 0.0);

		assert_effects_match(legacy_effects, optimized_effects);
	}

	#[test]
	fn transition_at_one_returns_target_effects_with_partial_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let femur = rig.leg(Side::Left).femur.name.to_string();
		let from_pose = partial_from_pose(&rig, &[femur.as_str()]);
		let target = Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0);
		let transition = Transition::from_pose(target.clone(), from_pose);

		let mut expected = HumanoidV0Rig::imported();
		seed_bind_pose(&mut expected);
		let expected_effects = target.apply(&mut expected, 0.5);

		let mut legacy_rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut legacy_rig);
		let legacy_effects = apply_legacy(&transition, &mut legacy_rig, 0.5, 1.0);

		let mut optimized_rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut optimized_rig);
		let optimized_effects = transition.apply(&mut optimized_rig, 0.5, 1.0);

		assert_effects_match(expected_effects, legacy_effects);
		assert_effects_match(expected_effects, optimized_effects);
		assert!(expected_effects.r#move.is_some());
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

	#[derive(Clone, Copy)]
	enum BenchCase {
		WeightZero,
		WeightOne,
		Intermediate,
	}

	impl BenchCase {
		fn transition_progress(self) -> f32 {
			match self {
				Self::WeightZero => 0.0,
				Self::WeightOne => 1.0,
				Self::Intermediate => 0.5,
			}
		}

		fn label(self) -> &'static str {
			match self {
				Self::WeightZero => "weight_0",
				Self::WeightOne => "weight_1",
				Self::Intermediate => "weight_0.5",
			}
		}
	}

	fn bench_case(case: BenchCase, use_legacy: bool) -> (u128, u128, u128) {
		const FRAMES: u32 = 5_000;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;
		let transition_progress = case.transition_progress();

		let mut rigs: Vec<_> = (0..CHARACTERS)
			.map(|_| {
				let mut rig = HumanoidV0Rig::imported();
				seed_bind_pose(&mut rig);
				let from_pose = capture_animation_pose(
					&Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0),
					&mut rig,
					0.0,
				);
				(rig, Transition::from_pose(Spring::<HumanoidV0Rig>::default(), from_pose))
			})
			.collect();

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let animation_progress = black_box((frame % 100) as f32 / 100.0);
				for (rig, transition) in &mut rigs {
					let effects = if use_legacy {
						black_box(apply_legacy(
							transition,
							rig,
							animation_progress,
							black_box(transition_progress),
						))
					} else {
						black_box(transition.apply(
							rig,
							animation_progress,
							black_box(transition_progress),
						))
					};
					black_box(effects);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}

		run_ns.sort_unstable();
		let min = *run_ns.first().expect("run");
		let median = run_ns[run_ns.len() / 2];
		(min, median, run_ns.iter().sum::<u128>() / run_ns.len() as u128)
	}

	fn report_bench(label: &str, min: u128, median: u128, mean: u128) {
		eprintln!(
			"transition_apply_microbench {label}: min={min} ns/sample median={median} ns/sample mean={mean} ns/sample"
		);
	}

	/// Micro-benchmark for [`Transition::apply`] on a jump-style blend (Spring segment).
	/// Run with:
	/// `cargo test -p character-animations transition_apply_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn transition_apply_microbench() {
		eprintln!("32 characters × 5000 frames × 5 runs per case");
		for case in [BenchCase::WeightZero, BenchCase::WeightOne, BenchCase::Intermediate] {
			let (legacy_min, legacy_median, legacy_mean) = bench_case(case, true);
			report_bench(
				&format!("{} legacy", case.label()),
				legacy_min,
				legacy_median,
				legacy_mean,
			);
			let (min, median, mean) = bench_case(case, false);
			report_bench(&format!("{} optimized", case.label()), min, median, mean);
		}
	}
}
