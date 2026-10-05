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

	/// Write bone pose only. Skips child [`Animation::effects_for`] and effect mixing.
	pub fn apply_pose_for(&self, rig: &mut R, animation_progress: f32, transition_progress: f32) {
		self.animation.apply_for(rig, animation_progress);
		let target_pose = snapshot_pose(rig);
		let weight = self.weight(transition_progress);
		blend_pose(rig, &self.from_pose, &target_pose, weight);
	}

	/// Compatibility: pose via [`Self::apply_pose_for`], then child effects mixed by weight.
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
	use std::hint::black_box;
	use std::time::Instant;

	use character_rigs::{rigs::humanoid_v0::HumanoidV0Rig, RigPose, Side};

	use super::*;
	use crate::animations::{Fall, Land, Spring, Squat, Transition, TransitionCurve};
	use crate::rigs::mix::seed_bind_pose;
	use crate::Effects;

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

	fn assert_pose_matches_apply_at_weight(
		from_pose: RigPose,
		animation_progress: f32,
		transition_progress: f32,
	) {
		let target = Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0);
		let transition = Transition::from_pose(target, from_pose);

		let mut apply_rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut apply_rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut apply_rig, 0.25);
		let mut pose_only_rig = clone_rig(&apply_rig);

		let _ = transition.apply(&mut apply_rig, animation_progress, transition_progress);
		transition.apply_pose_for(&mut pose_only_rig, animation_progress, transition_progress);

		assert_poses_match(&apply_rig, &pose_only_rig);
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

	#[test]
	fn apply_pose_for_matches_apply_pose_at_weight_zero_full_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut rig, 1.0);
		assert_pose_matches_apply_at_weight(snapshot_pose(&rig), 0.5, 0.0);
	}

	#[test]
	fn apply_pose_for_matches_apply_pose_at_weight_one_full_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		assert_pose_matches_apply_at_weight(snapshot_pose(&rig), 0.5, 1.0);
	}

	#[test]
	fn apply_pose_for_matches_apply_pose_at_weight_half_full_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut rig, 0.5);
		assert_pose_matches_apply_at_weight(snapshot_pose(&rig), 0.5, 0.5);
	}

	#[test]
	fn apply_pose_for_matches_apply_pose_at_weight_zero_partial_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut rig, 1.0);
		let femur = rig.leg(Side::Left).femur.name.to_string();
		assert_pose_matches_apply_at_weight(partial_from_pose(&rig, &[femur.as_str()]), 0.5, 0.0);
	}

	#[test]
	fn apply_pose_for_matches_apply_pose_at_weight_one_partial_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut rig, 0.5);
		let femur = rig.leg(Side::Left).femur.name.to_string();
		let shoulder = rig.arm(Side::Left).shoulder.name.to_string();
		assert_pose_matches_apply_at_weight(
			partial_from_pose(&rig, &[femur.as_str(), shoulder.as_str()]),
			0.5,
			1.0,
		);
	}

	#[test]
	fn apply_returns_unchanged_effects_at_weight_zero_partial_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let femur = rig.leg(Side::Left).femur.name.to_string();
		let from_pose = partial_from_pose(&rig, &[femur.as_str()]);
		let transition =
			Transition::from_pose(Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0), from_pose);

		let mut rig_a = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig_a);
		let effects_a = transition.apply(&mut rig_a, 0.5, 0.0);

		let mut rig_b = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig_b);
		let effects_b = transition.apply(&mut rig_b, 0.5, 0.0);

		assert_effects_match(effects_a, effects_b);
		assert_poses_match(&rig_a, &rig_b);
	}

	#[test]
	fn apply_returns_unchanged_effects_at_weight_one_partial_from_pose() {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let femur = rig.leg(Side::Left).femur.name.to_string();
		let from_pose = partial_from_pose(&rig, &[femur.as_str()]);
		let target = Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0);
		let transition = Transition::from_pose(target.clone(), from_pose);

		let mut expected = HumanoidV0Rig::imported();
		seed_bind_pose(&mut expected);
		let expected_effects = target.apply(&mut expected, 0.5);

		let mut rig_a = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig_a);
		let effects_a = transition.apply(&mut rig_a, 0.5, 1.0);

		let mut rig_b = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig_b);
		let effects_b = transition.apply(&mut rig_b, 0.5, 1.0);

		assert_effects_match(expected_effects, effects_a);
		assert_effects_match(expected_effects, effects_b);
		assert!(expected_effects.r#move.is_some());
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

	fn bench_pose_case(case: BenchCase, use_apply: bool) -> (u128, u128, u128) {
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
					if use_apply {
						let _ = black_box(transition.apply(
							rig,
							animation_progress,
							black_box(transition_progress),
						));
					} else {
						black_box(transition.apply_pose_for(
							rig,
							animation_progress,
							black_box(transition_progress),
						));
					}
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
			"transition_pose_microbench {label}: min={min} ns/sample median={median} ns/sample mean={mean} ns/sample"
		);
	}

	/// Micro-benchmark for [`Transition::apply_pose_for`] vs [`Transition::apply`] pose path.
	/// Run with:
	/// `cargo test -p character-animations transition_pose_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn transition_pose_microbench() {
		eprintln!("32 characters × 5000 frames × 5 runs per case (Spring transition)");
		for case in [BenchCase::WeightZero, BenchCase::WeightOne, BenchCase::Intermediate] {
			let (apply_min, apply_median, apply_mean) = bench_pose_case(case, true);
			report_bench(
				&format!("{} apply (discards effects)", case.label()),
				apply_min,
				apply_median,
				apply_mean,
			);
			let (min, median, mean) = bench_pose_case(case, false);
			report_bench(&format!("{} apply_pose_for", case.label()), min, median, mean);
		}
	}
}
