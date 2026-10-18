use character_rigs::authoring::{ArmatureOffset, PoseBuffer};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{Mix, Smooth};
use crate::{Animation, Effects};

impl<A, B> Animation<HumanoidV0Rig> for Mix<A, B>
where
	A: Animation<HumanoidV0Rig>,
	B: Animation<HumanoidV0Rig>,
{
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		blend_poses(rig, &self.from, &self.to, progress, progress, self.weight);
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, progress: f32) -> Effects {
		let weight = self.weight.clamp(0.0, 1.0);
		if weight <= 0.0 {
			return self.from.effects_for(rig, progress);
		}
		if weight >= 1.0 {
			return self.to.effects_for(rig, progress);
		}
		mix_effects(
			self.from.effects_for(rig, progress),
			self.to.effects_for(rig, progress),
			weight,
		)
	}
}

impl<A, B> Mix<A, B>
where
	A: Animation<HumanoidV0Rig>,
	B: Animation<HumanoidV0Rig>,
{
	pub fn apply_at(
		&self,
		rig: &mut HumanoidV0Rig,
		from_progress: f32,
		to_progress: f32,
	) -> Effects {
		blend_animations(rig, &self.from, &self.to, from_progress, to_progress, self.weight)
	}
}

impl<A, B> Animation<HumanoidV0Rig> for Smooth<A, B>
where
	A: Animation<HumanoidV0Rig>,
	B: Animation<HumanoidV0Rig>,
{
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		blend_poses(
			rig,
			&self.from,
			&self.to,
			progress,
			progress,
			crate::animations::smoothstep(self.weight),
		);
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, progress: f32) -> Effects {
		let weight = crate::animations::smoothstep(self.weight);
		if weight <= 0.0 {
			return self.from.effects_for(rig, progress);
		}
		if weight >= 1.0 {
			return self.to.effects_for(rig, progress);
		}
		mix_effects(
			self.from.effects_for(rig, progress),
			self.to.effects_for(rig, progress),
			weight,
		)
	}
}

impl<A, B> Smooth<A, B>
where
	A: Animation<HumanoidV0Rig>,
	B: Animation<HumanoidV0Rig>,
{
	pub fn apply_at(
		&self,
		rig: &mut HumanoidV0Rig,
		from_progress: f32,
		to_progress: f32,
	) -> Effects {
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

fn blend_animations<A, B>(
	rig: &mut HumanoidV0Rig,
	from: &A,
	to: &B,
	from_progress: f32,
	to_progress: f32,
	weight: f32,
) -> Effects
where
	A: Animation<HumanoidV0Rig>,
	B: Animation<HumanoidV0Rig>,
{
	let weight = weight.clamp(0.0, 1.0);
	blend_poses(rig, from, to, from_progress, to_progress, weight);
	if weight <= 0.0 {
		return from.effects_for(rig, from_progress);
	}
	if weight >= 1.0 {
		return to.effects_for(rig, to_progress);
	}
	mix_effects(from.effects_for(rig, from_progress), to.effects_for(rig, to_progress), weight)
}

/// Sample two clips into the rig scratch and blend transforms. Weight 0 keeps `from`.
pub(crate) fn blend_clips<A, B>(
	rig: &mut HumanoidV0Rig,
	from: &A,
	from_progress: f32,
	to: &B,
	to_progress: f32,
	weight: f32,
) where
	A: Animation<HumanoidV0Rig>,
	B: Animation<HumanoidV0Rig>,
{
	blend_poses(rig, from, to, from_progress, to_progress, weight);
}

fn blend_poses<A, B>(
	rig: &mut HumanoidV0Rig,
	from: &A,
	to: &B,
	from_progress: f32,
	to_progress: f32,
	weight: f32,
) where
	A: Animation<HumanoidV0Rig>,
	B: Animation<HumanoidV0Rig>,
{
	let weight = weight.clamp(0.0, 1.0);
	if weight <= 0.0 {
		from.apply_for(rig, from_progress);
		return;
	}
	if weight >= 1.0 {
		to.apply_for(rig, to_progress);
		return;
	}

	let depth = rig.scratch.depth;
	rig.scratch.depth = depth + 1;
	from.apply_for(rig, from_progress);
	rig.scratch.capture_from(depth, &rig.pose);
	to.apply_for(rig, to_progress);
	rig.scratch.capture_to(depth, &rig.pose);
	rig.scratch.blend_saved(depth, weight, &mut rig.pose);
	rig.scratch.depth = depth;
}

pub(crate) fn mix_effects(from: Effects, to: Effects, weight: f32) -> Effects {
	ArmatureOffset::blend(from, to, weight)
}

/// Samples an animation into a pose buffer. Copies into `dest` without growing it
/// when the length already matches.
pub(crate) fn capture_into<A: Animation<HumanoidV0Rig>>(
	anim: &A,
	rig: &mut HumanoidV0Rig,
	progress: f32,
	dest: &mut PoseBuffer,
) {
	anim.apply_for(rig, progress);
	dest.copy_from(&rig.pose);
}

#[cfg(test)]
mod tests {
	use std::hint::black_box;
	use std::time::Instant;

	use bevy::prelude::*;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::{Mix, Smooth, Spring, Squat, TwoFootedJump};
	use crate::Animation;

	fn blend_poses_legacy<A, B>(
		rig: &mut HumanoidV0Rig,
		from: &A,
		to: &B,
		from_progress: f32,
		to_progress: f32,
		weight: f32,
	) where
		A: Animation<HumanoidV0Rig>,
		B: Animation<HumanoidV0Rig>,
	{
		let depth = rig.scratch.depth;
		rig.scratch.depth = depth + 1;
		from.apply_for(rig, from_progress);
		rig.scratch.capture_from(depth, &rig.pose);
		to.apply_for(rig, to_progress);
		rig.scratch.capture_to(depth, &rig.pose);
		rig.scratch.blend_saved(depth, weight.clamp(0.0, 1.0), &mut rig.pose);
		rig.scratch.depth = depth;
	}

	fn assert_pose_near(a: &HumanoidV0Rig, b: &HumanoidV0Rig, label: &str) {
		for name in a.animation_bone_names() {
			assert!(a.rotation(name).dot(b.rotation(name)).abs() > 1.0 - 1e-5, "{label}: {name}");
		}
	}

	#[test]
	fn blend_poses_endpoints_match_legacy() -> anyhow::Result<()> {
		let squat = Squat::for_loop(1.0, 1.0);
		let spring = Spring::default();
		let cases = [(0.0, 0.25, 0.75, 0.0), (0.0, 0.25, 0.75, 1.0), (0.25, 0.5, 0.75, 0.5)];
		for (from_p, to_p, weight, _) in cases {
			let mut legacy = HumanoidV0Rig::imported();
			blend_poses_legacy(&mut legacy, &squat, &spring, from_p, to_p, weight);
			let mut optimized = HumanoidV0Rig::imported();
			blend_poses(&mut optimized, &squat, &spring, from_p, to_p, weight);
			assert_pose_near(&legacy, &optimized, "pose");
		}
		Ok(())
	}

	#[test]
	fn mix_apply_at_endpoints_match_legacy_effects() -> anyhow::Result<()> {
		let mix = Mix::new(Squat::for_loop(1.0, 1.0), Spring::default(), 0.0);
		let mut legacy = HumanoidV0Rig::imported();
		let legacy_effects = {
			blend_poses_legacy(&mut legacy, &mix.from, &mix.to, 0.2, 0.8, mix.weight);
			mix_effects(
				mix.from.effects_for(&legacy, 0.2),
				mix.to.effects_for(&legacy, 0.8),
				mix.weight,
			)
		};
		let mut optimized = HumanoidV0Rig::imported();
		let optimized_effects = mix.apply_at(&mut optimized, 0.2, 0.8);
		assert_pose_near(&legacy, &optimized, "weight 0");
		assert_eq!(legacy_effects, optimized_effects);

		let mix = Mix::new(Squat::for_loop(1.0, 1.0), Spring::default(), 1.0);
		let mut legacy = HumanoidV0Rig::imported();
		let legacy_effects = {
			blend_poses_legacy(&mut legacy, &mix.from, &mix.to, 0.2, 0.8, mix.weight);
			mix_effects(
				mix.from.effects_for(&legacy, 0.2),
				mix.to.effects_for(&legacy, 0.8),
				mix.weight,
			)
		};
		let mut optimized = HumanoidV0Rig::imported();
		let optimized_effects = mix.apply_at(&mut optimized, 0.2, 0.8);
		assert_pose_near(&legacy, &optimized, "weight 1");
		assert_eq!(legacy_effects, optimized_effects);
		Ok(())
	}

	fn bench_mix_pose_blend(legacy: bool, weight: f32) -> Vec<u128> {
		const FRAMES: u32 = 5_000;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;
		let mix = Mix::new(Squat::for_loop(1.0, 1.0), Spring::default(), weight);
		let mut rigs: Vec<HumanoidV0Rig> =
			(0..CHARACTERS).map(|_| HumanoidV0Rig::imported()).collect();

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (index, rig) in rigs.iter_mut().enumerate() {
					let from_p =
						black_box((frame as f32 * 0.011 + index as f32 * 0.03).rem_euclid(1.0));
					let to_p =
						black_box((frame as f32 * 0.017 + index as f32 * 0.05).rem_euclid(1.0));
					if legacy {
						blend_poses_legacy(rig, &mix.from, &mix.to, from_p, to_p, mix.weight);
					} else {
						blend_poses(rig, &mix.from, &mix.to, from_p, to_p, mix.weight);
					}
					black_box(&rig.pose);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}
		run_ns.sort_unstable();
		run_ns
	}

	fn bench_jump_apply_for(legacy: bool) -> Vec<u128> {
		const FRAMES: u32 = 500;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;
		let jump = TwoFootedJump::default();
		let cycle = {
			let rig = HumanoidV0Rig::imported();
			jump.timings(rig.segment_lengths).cycle_duration()
		};
		let squat_start = {
			let rig = HumanoidV0Rig::imported();
			jump.timings(rig.segment_lengths).squat_end()
		};
		let mut rigs: Vec<(HumanoidV0Rig, f32)> =
			(0..CHARACTERS).map(|_| (HumanoidV0Rig::imported(), squat_start)).collect();

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let _frame = black_box(frame);
				for (rig, elapsed) in &mut rigs {
					let dt = black_box(0.016);
					*elapsed += dt;
					if *elapsed > cycle {
						*elapsed = squat_start;
					}
					if legacy {
						blend_clips_legacy(rig, &jump, *elapsed);
					} else {
						jump.apply_for(rig, *elapsed);
					}
					black_box(&rig.pose);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}
		run_ns.sort_unstable();
		run_ns
	}

	fn blend_clips_legacy(rig: &mut HumanoidV0Rig, jump: &TwoFootedJump, elapsed: f32) {
		use crate::animations::{Fall, JumpSegment, Spring, Squat, FALL_BLEND_FRACTION};
		let lengths = rig.segment_lengths;
		let (segment, local) = jump.segment(lengths, elapsed);
		let timings = jump.timings(lengths);
		match segment {
			JumpSegment::Squat => {
				let squat = jump.prejump_squat(lengths);
				let progress = local / timings.squat_duration().max(f32::EPSILON);
				squat.apply_for(rig, progress);
			}
			JumpSegment::Spring => blend_poses_legacy(
				rig,
				&Squat::for_loop(1.0, 1.0),
				&Spring::default(),
				0.0,
				local,
				crate::animations::smoothstep(local),
			),
			JumpSegment::Fall => {
				let fall = Fall::default();
				let blend_end = FALL_BLEND_FRACTION;
				if local < blend_end {
					let transition_progress = (local / blend_end).clamp(0.0, 1.0);
					blend_poses_legacy(
						rig,
						&Spring::default(),
						&fall,
						1.0,
						local,
						crate::animations::smoothstep(transition_progress),
					);
				} else {
					fall.apply_for(rig, local);
				}
			}
			JumpSegment::Land => {
				let land = jump.landing_squat(lengths);
				let land_duration = timings.land_duration().max(f32::EPSILON);
				let land_progress = local / land_duration;
				let blend_window = timings.land_pose_blend_duration();
				let transition_progress = if blend_window > f32::EPSILON {
					(local / blend_window).clamp(0.0, 1.0)
				} else {
					1.0
				};
				if transition_progress < 1.0 {
					blend_poses_legacy(
						rig,
						&Fall::default(),
						&land,
						1.0,
						land_progress,
						crate::animations::smoothstep(transition_progress),
					);
				} else {
					land.apply_for(rig, land_progress);
				}
			}
		}
	}

	fn mix_effects_for_legacy<A, B>(mix: &Mix<A, B>, rig: &HumanoidV0Rig, progress: f32) -> Effects
	where
		A: Animation<HumanoidV0Rig>,
		B: Animation<HumanoidV0Rig>,
	{
		mix_effects(
			mix.from.effects_for(rig, progress),
			mix.to.effects_for(rig, progress),
			mix.weight,
		)
	}

	fn bench_mix_effects_for(legacy: bool, weight: f32) -> Vec<u128> {
		const FRAMES: u32 = 5_000;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;
		let mix = Mix::new(Squat::for_loop(1.0, 1.0), TwoFootedJump::default(), weight);
		let rigs: Vec<HumanoidV0Rig> = (0..CHARACTERS).map(|_| HumanoidV0Rig::imported()).collect();
		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (index, rig) in rigs.iter().enumerate() {
					let progress =
						black_box((frame as f32 * 0.011 + index as f32 * 0.03).rem_euclid(1.0));
					if legacy {
						black_box(mix_effects_for_legacy(&mix, rig, progress));
					} else {
						black_box(mix.effects_for(rig, progress));
					}
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}
		run_ns.sort_unstable();
		run_ns
	}

	#[test]
	fn mix_effects_for_endpoints_match_legacy() -> anyhow::Result<()> {
		let squat = Squat::for_loop(1.0, 1.0);
		let jump = TwoFootedJump::default();
		let rig = HumanoidV0Rig::imported();
		for weight in [0.0, 1.0, 0.5] {
			let mix = Mix::new(squat.clone(), jump.clone(), weight);
			for progress in [0.0, 0.35, 0.9] {
				let legacy = mix_effects_for_legacy(&mix, &rig, progress);
				let optimized = mix.effects_for(&rig, progress);
				assert_eq!(legacy, optimized, "weight={weight} progress={progress}");
			}
		}
		Ok(())
	}

	fn report_bench(label: &str, run_ns: &[u128]) {
		let min = *run_ns.first().expect("run");
		let median = run_ns[run_ns.len() / 2];
		let mean = run_ns.iter().sum::<u128>() / run_ns.len() as u128;
		eprintln!(
			"mix_blend_microbench {label}: runs={runs:?} min={min} median={median} mean={mean} ns/sample",
			runs = run_ns,
		);
	}

	/// `cargo test -p character-animations mix_effects_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn mix_effects_microbench() {
		eprintln!(
			"32 characters × 5000 frames × 5 runs, Mix<Squat,TwoFootedJump> effects_for only"
		);
		report_bench("weight 0.0 legacy", &bench_mix_effects_for(true, 0.0));
		report_bench("weight 0.0 optimized", &bench_mix_effects_for(false, 0.0));
		report_bench("weight 1.0 legacy", &bench_mix_effects_for(true, 1.0));
		report_bench("weight 1.0 optimized", &bench_mix_effects_for(false, 1.0));
		report_bench("weight 0.5 legacy", &bench_mix_effects_for(true, 0.5));
		report_bench("weight 0.5 optimized", &bench_mix_effects_for(false, 0.5));
	}

	/// `cargo test -p character-animations mix_blend_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn mix_blend_microbench() {
		eprintln!("32 characters × 5000 frames × 5 runs, Mix<Squat,Spring> pose blend");
		report_bench("weight 0.0 legacy", &bench_mix_pose_blend(true, 0.0));
		report_bench("weight 0.0 optimized", &bench_mix_pose_blend(false, 0.0));
		report_bench("weight 1.0 legacy", &bench_mix_pose_blend(true, 1.0));
		report_bench("weight 1.0 optimized", &bench_mix_pose_blend(false, 1.0));
		report_bench("weight 0.5 legacy", &bench_mix_pose_blend(true, 0.5));
		report_bench("weight 0.5 optimized", &bench_mix_pose_blend(false, 0.5));
		eprintln!("32 characters advancing TwoFootedJump through spring/fall/land blends");
		report_bench("jump apply_for legacy blend", &bench_jump_apply_for(true));
		report_bench("jump apply_for optimized blend", &bench_jump_apply_for(false));
	}

	#[test]
	fn mix_interpolates_femur_swing() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let mix = Mix::new(Squat::for_loop(1.0, 1.0), Squat::for_loop(1.0, 1.0), 0.5);
		mix.apply_at(&mut rig, 0.0, 0.5);

		assert!(rig.posed_angle("femur.L") > 0.0);
		let mut full = HumanoidV0Rig::imported();
		Squat::for_loop(1.0, 1.0).apply(&mut full, 0.5);
		assert!(rig.posed_angle("femur.L") < full.posed_angle("femur.L"));
		Ok(())
	}

	#[test]
	fn mix_at_zero_matches_from() -> anyhow::Result<()> {
		let mut rig_a = HumanoidV0Rig::imported();
		let mut rig_b = HumanoidV0Rig::imported();

		Squat::for_loop(1.0, 1.0).apply(&mut rig_a, 0.25);
		Mix::new(Squat::for_loop(1.0, 1.0), Squat::for_loop(1.0, 1.0), 0.0)
			.apply_at(&mut rig_b, 0.25, 0.75);

		assert!(rig_a.rotation("femur.L").dot(rig_b.rotation("femur.L")).abs() > 1.0 - 1e-5);
		Ok(())
	}

	#[test]
	fn identity_offset_blend_fades_rotation() {
		let from = ArmatureOffset::IDENTITY;
		let to = ArmatureOffset::from_rotation(Quat::from_rotation_x(0.8));
		let mid = mix_effects(from, to, 0.5);
		assert!(mid.0.rotation.dot(Quat::IDENTITY).abs() < 0.999);
		assert!(mix_effects(from, to, 0.0).is_identity());
		assert!(mix_effects(from, to, 1.0).0.rotation.dot(to.0.rotation).abs() > 1.0 - 1e-5);
	}

	#[test]
	fn nested_mix_keeps_the_outer_first_child() -> anyhow::Result<()> {
		let squat = Squat::for_loop(1.0, 1.0);
		let inner = Mix::new(Spring::default(), Squat::for_loop(1.0, 1.0), 0.5);
		let nested = Mix::new(squat.clone(), inner.clone(), 0.5);

		let mut expected_from = HumanoidV0Rig::imported();
		squat.apply(&mut expected_from, 0.25);
		let mut expected_to = HumanoidV0Rig::imported();
		inner.apply_at(&mut expected_to, 0.25, 0.25);
		let mut expected = HumanoidV0Rig::imported();
		PoseBuffer::blend_into(&expected_from.pose, &expected_to.pose, 0.5, &mut expected.pose);

		let mut nested_rig = HumanoidV0Rig::imported();
		nested.apply_at(&mut nested_rig, 0.25, 0.25);

		assert!(
			expected.rotation("femur.L").dot(nested_rig.rotation("femur.L")).abs() > 1.0 - 1e-5,
			"inner Mix must not overwrite the outer from-pose"
		);
		assert!(
			expected.rotation("root").dot(nested_rig.rotation("root")).abs() > 1.0 - 1e-5,
			"spine blend must keep both children"
		);
		Ok(())
	}

	#[test]
	fn smooth_spring_from_stand_blends_arms() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Smooth::new(Squat::for_loop(1.0, 1.0), Spring::default(), 0.5).apply_at(&mut rig, 0.0, 1.0);

		assert!(rig.posed_angle("shoulder.L") > 0.02, "blended shoulder leaves rest");
		Ok(())
	}

	fn lerp_channel(a: f32, b: f32, t: f32) -> f32 {
		a + (b - a) * t
	}

	fn lerp_array<const N: usize>(a: [f32; N], b: [f32; N], t: f32) -> [f32; N] {
		std::array::from_fn(|i| lerp_channel(a[i], b[i], t))
	}

	fn lerp_humanoid_pose(
		from: character_rigs::authoring::HumanoidPose,
		to: character_rigs::authoring::HumanoidPose,
		weight: f32,
	) -> character_rigs::authoring::HumanoidPose {
		use character_rigs::authoring::{
			ArmPose, HumanoidPose, LegPose, NeckBone, NeckPose, SpinePose,
		};
		let t = weight.clamp(0.0, 1.0);
		let lerp_neck = |a: NeckBone, b: NeckBone| NeckBone {
			nod: lerp_channel(a.nod, b.nod, t),
			side_tilt: lerp_channel(a.side_tilt, b.side_tilt, t),
			turn: lerp_channel(a.turn, b.turn, t),
		};
		let lerp_leg = |a: LegPose, b: LegPose| LegPose {
			hip_flexion: lerp_channel(a.hip_flexion, b.hip_flexion, t),
			hip_abduction: lerp_channel(a.hip_abduction, b.hip_abduction, t),
			hip_rotation: lerp_channel(a.hip_rotation, b.hip_rotation, t),
			knee_flexion: lerp_channel(a.knee_flexion, b.knee_flexion, t),
			pelvis_flexion: lerp_channel(a.pelvis_flexion, b.pelvis_flexion, t),
			pelvis_lateral: lerp_channel(a.pelvis_lateral, b.pelvis_lateral, t),
			pelvis_turn: lerp_channel(a.pelvis_turn, b.pelvis_turn, t),
		};
		let lerp_arm = |a: ArmPose, b: ArmPose| ArmPose {
			forward_elevation: lerp_channel(a.forward_elevation, b.forward_elevation, t),
			lateral_elevation: lerp_channel(a.lateral_elevation, b.lateral_elevation, t),
			axial_rotation: lerp_channel(a.axial_rotation, b.axial_rotation, t),
			elbow_flexion: lerp_channel(a.elbow_flexion, b.elbow_flexion, t),
			shoulder_forward: lerp_channel(a.shoulder_forward, b.shoulder_forward, t),
			shoulder_lift: lerp_channel(a.shoulder_lift, b.shoulder_lift, t),
			shoulder_twist: lerp_channel(a.shoulder_twist, b.shoulder_twist, t),
			aim: if t >= 0.5 { b.aim } else { a.aim },
		};
		HumanoidPose {
			spine: SpinePose {
				forward_bend: lerp_array(from.spine.forward_bend, to.spine.forward_bend, t),
				side_bend: lerp_array(from.spine.side_bend, to.spine.side_bend, t),
				turn: lerp_array(from.spine.turn, to.spine.turn, t),
			},
			neck: NeckPose {
				lower: lerp_neck(from.neck.lower, to.neck.lower),
				upper: lerp_neck(from.neck.upper, to.neck.upper),
			},
			legs: [lerp_leg(from.legs[0], to.legs[0]), lerp_leg(from.legs[1], to.legs[1])],
			arms: [lerp_arm(from.arms[0], to.arms[0]), lerp_arm(from.arms[1], to.arms[1])],
		}
	}

	fn blend_authored_then_resolve(
		rig: &mut HumanoidV0Rig,
		from: character_rigs::authoring::HumanoidPose,
		to: character_rigs::authoring::HumanoidPose,
		weight: f32,
	) {
		let weight = weight.clamp(0.0, 1.0);
		if weight <= 0.0 {
			rig.write_pose(&from);
			return;
		}
		if weight >= 1.0 {
			rig.write_pose(&to);
			return;
		}
		rig.write_pose(&lerp_humanoid_pose(from, to, weight));
	}

	fn apply_jump_authored_blend(jump: &TwoFootedJump, rig: &mut HumanoidV0Rig, elapsed: f32) {
		use crate::animations::{Fall, JumpSegment, Spring, Squat};
		let sample = jump.sample(rig.segment_lengths, elapsed);
		match sample.segment {
			JumpSegment::Squat => sample.windup.apply_for(rig, sample.pose_progress),
			JumpSegment::Spring => blend_authored_then_resolve(
				rig,
				Squat::for_loop(1.0, 1.0).sample_pose(0.0),
				Spring::default().sample_pose(sample.local),
				crate::animations::smoothstep(sample.blend_progress),
			),
			JumpSegment::Fall => {
				if sample.blend_progress < 1.0 {
					blend_authored_then_resolve(
						rig,
						Spring::default().sample_pose(1.0),
						Fall::default().sample_pose(sample.local),
						crate::animations::smoothstep(sample.blend_progress),
					);
				} else {
					Fall::default().apply_for(rig, sample.local);
				}
			}
			JumpSegment::Land => {
				if sample.blend_progress < 1.0 {
					blend_authored_then_resolve(
						rig,
						Fall::default().sample_pose(1.0),
						sample.landing.sample_pose(sample.pose_progress),
						crate::animations::smoothstep(sample.blend_progress),
					);
				} else {
					sample.landing.apply_for(rig, sample.pose_progress);
				}
			}
		}
	}

	fn max_pose_angle_delta(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).angle_between(b.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	#[test]
	fn authored_pose_blend_numerical_delta_from_resolved_slerp() -> anyhow::Result<()> {
		let jump = TwoFootedJump::default();
		let lengths = HumanoidV0Rig::imported().segment_lengths;
		let timings = jump.timings(lengths);
		let blend_times = [
			timings.squat_end() + timings.spring_duration * 0.5,
			timings.spring_end()
				+ timings.air_duration * crate::animations::FALL_BLEND_FRACTION * 0.5,
			timings.air_end() + timings.land_pose_blend_duration() * 0.5,
		];
		let mut max_delta = 0.0_f32;
		for elapsed in blend_times {
			let mut resolved = HumanoidV0Rig::imported();
			jump.apply_for(&mut resolved, elapsed);
			let mut authored = HumanoidV0Rig::imported();
			apply_jump_authored_blend(&jump, &mut authored, elapsed);
			max_delta = max_delta.max(max_pose_angle_delta(&resolved, &authored));
		}
		eprintln!(
			"authored_pose_blend max bone angle delta vs resolved slerp: {max_delta:.6} rad ({:.4} deg)",
			max_delta.to_degrees()
		);
		if max_delta <= 1e-4 {
			eprintln!(
				"authored_pose_blend: numerically equivalent at sampled blend windows (delta={max_delta})"
			);
		}
		Ok(())
	}

	fn bench_jump_authored_vs_resolved(authored: bool) -> Vec<u128> {
		const FRAMES: u32 = 400;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 7;
		let jump = TwoFootedJump::default();
		let cycle = {
			let rig = HumanoidV0Rig::imported();
			jump.timings(rig.segment_lengths).cycle_duration()
		};
		let mut rigs: Vec<HumanoidV0Rig> =
			(0..CHARACTERS).map(|_| HumanoidV0Rig::imported()).collect();
		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (index, rig) in rigs.iter_mut().enumerate() {
					let elapsed = black_box(
						(frame as f32 * 0.016 + index as f32 * 0.11) % cycle.max(f32::EPSILON),
					);
					if authored {
						apply_jump_authored_blend(&jump, rig, elapsed);
					} else {
						jump.apply_for(rig, elapsed);
					}
					black_box(&rig.pose);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}
		run_ns.sort_unstable();
		run_ns
	}

	/// `cargo test -p character-animations authored_pose_blend_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn authored_pose_blend_microbench() {
		eprintln!(
			"32 characters × 400 frames × 7 runs; complete jump cycles; resolved slerp vs authored lerp then resolve once"
		);
		report_bench(
			"resolved quaternion slerp (production)",
			&bench_jump_authored_vs_resolved(false),
		);
		report_bench("authored pose lerp then resolve", &bench_jump_authored_vs_resolved(true));
	}
}
