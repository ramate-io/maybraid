use bevy::prelude::{Transform, Vec3};
use character_rigs::{humanoid::HumanoidRig, Side};
use log::info;

use crate::animations::{
	Fall, JumpSegment, Spring, Squat, Transition, TransitionCurve, TwoFootedJump,
	FALL_BLEND_FRACTION,
};
use crate::rigs::jump_transition_cache::{
	JumpTransitionCache, JumpTransitionKey, MaskedTransitionSource,
};
use crate::rigs::transition::{apply_with_from_pose, capture_animation_pose};
use crate::{Animation, Effects};

fn segment_debug_enabled() -> bool {
	std::env::var("CROZON_ANIMATION_DEBUG").is_ok()
}

impl<R: HumanoidRig> Animation<R> for TwoFootedJump<R> {
	fn apply_for(&self, rig: &mut R, elapsed: f32) {
		self.apply_for_inner(rig, elapsed, None);
	}

	fn effects_for(&self, rig: &R, elapsed: f32) -> Effects {
		let lengths = rig.segment_lengths();
		let y = self.vertical_offset(lengths, elapsed);
		Effects {
			r#move: (y.abs() > f32::EPSILON)
				.then(|| Transform::from_translation(Vec3::new(0.0, y, 0.0))),
		}
	}
}

impl<R: HumanoidRig> TwoFootedJump<R> {
	/// Sample the jump with a per-character transition cache (mailbox playback).
	pub fn apply_for_with_cache(&self, rig: &mut R, elapsed: f32, cache: &mut JumpTransitionCache) {
		cache.note_elapsed(elapsed);
		self.apply_for_inner(rig, elapsed, Some(cache));
	}

	fn apply_for_inner(&self, rig: &mut R, elapsed: f32, cache: Option<&mut JumpTransitionCache>) {
		let lengths = rig.segment_lengths();
		let (segment, local) = self.segment(lengths, elapsed);
		let timings = self.timings(lengths);

		match segment {
			JumpSegment::Squat => {
				let squat = self.prejump_squat(lengths);
				let progress = local / timings.squat_duration().max(f32::EPSILON);
				squat.apply_for(rig, progress);
			}
			JumpSegment::Spring => {
				let spring = Spring::<R>::default();
				let transition = Transition::from_pose(spring, RigPose::new())
					.with_curve(TransitionCurve::SmoothStep);
				let _ = apply_jump_transition(
					cache,
					JumpTransitionKey::SpringFromSquat,
					rig,
					|r| capture_animation_pose(&Squat::<R>::for_loop(1.0, 1.0), r, 0.0),
					|r| MaskedTransitionSource::capture(&Squat::<R>::for_loop(1.0, 1.0), r, 0.0),
					&transition,
					local,
					local,
				);
			}
			JumpSegment::Fall => {
				let fall = Fall::<R>::default();
				let blend_end = FALL_BLEND_FRACTION;
				if segment_debug_enabled() && local > 0.9 {
					info!(
						"jump fall end: elapsed={:.3} cycle_t={:.3} air_end={:.3} fall_local={:.4} fall_femur=0 fall_shoulder_flex={:.4}",
						elapsed,
						self.time_in_cycle(lengths, elapsed),
						timings.air_end(),
						local,
						fall.shoulder_flex(Side::Left, local),
					);
				}
				if local < blend_end {
					let transition_progress = (local / blend_end).clamp(0.0, 1.0);
					let transition = Transition::from_pose(fall, RigPose::new())
						.with_curve(TransitionCurve::SmoothStep);
					let _ = apply_jump_transition(
						cache,
						JumpTransitionKey::FallFromSpring,
						rig,
						|r| capture_animation_pose(&Spring::<R>::default(), r, 1.0),
						|r| MaskedTransitionSource::capture(&Spring::<R>::default(), r, 1.0),
						&transition,
						local,
						transition_progress,
					);
				} else {
					if let Some(cache) = cache {
						cache.clear_after_segment(JumpSegment::Fall);
					}
					fall.apply_for(rig, local);
				}
			}
			JumpSegment::Land => {
				let land = self.landing_squat(lengths);
				let land_duration = timings.land_duration().max(f32::EPSILON);
				let land_progress = local / land_duration;
				let blend_window = timings.land_pose_blend_duration();
				let transition_progress = if blend_window > f32::EPSILON {
					(local / blend_window).clamp(0.0, 1.0)
				} else {
					1.0
				};
				if segment_debug_enabled() && local < timings.land_descent_duration + 0.05 {
					info!(
						"jump land start: elapsed={:.3} cycle_t={:.3} land_local={:.4} land_depth={:.4} land_femur={:.4} transition={:.4} land_desc_d={:.4} y={:.4}",
						elapsed,
						self.time_in_cycle(lengths, elapsed),
						local,
						land.depth(land_progress),
						land.femur_swing(land_progress),
						transition_progress,
						timings.land_descent_duration,
						self.vertical_offset(lengths, elapsed),
					);
				}
				if transition_progress < 1.0 {
					let transition = Transition::from_pose(land, RigPose::new())
						.with_curve(TransitionCurve::SmoothStep);
					let _ = apply_jump_transition(
						cache,
						JumpTransitionKey::LandFromFall,
						rig,
						|r| capture_animation_pose(&Fall::<R>::default(), r, 1.0),
						|r| MaskedTransitionSource::capture(&Fall::<R>::default(), r, 1.0),
						&transition,
						land_progress,
						transition_progress,
					);
				} else {
					if let Some(cache) = cache {
						cache.clear_after_segment(JumpSegment::Land);
					}
					land.apply_for(rig, land_progress);
				}
			}
		}
	}

	pub fn log_landing_debug(&self, rig: &R, elapsed: f32, label: &str) {
		let lengths = rig.segment_lengths();
		let timings = self.timings(lengths);
		let time_in_cycle = self.time_in_cycle(lengths, elapsed);
		let (segment, local) = self.segment(lengths, elapsed);
		let land = self.landing_squat(lengths);
		let land_progress = local / timings.land_duration().max(f32::EPSILON);
		let y = self.vertical_offset(lengths, elapsed);

		info!(
			"{label}: elapsed={:.3} cycle_t={:.3} segment={:?} local={:.4} land_depth={:.4} y={:.4} timings[squat=({:.3},{:.3}) spring={:.3} air={:.3} land=({:.4},{:.3})] speeds[pre={:.3} landing={:.3}]",
			elapsed,
			time_in_cycle,
			segment,
			local,
			land.depth(land_progress),
			y,
			timings.squat_descent_duration,
			timings.squat_ascent_duration,
			timings.spring_duration,
			timings.air_duration,
			timings.land_descent_duration,
			timings.land_ascent_duration,
			self.pre_squat_speed,
			self.landing_squat_speed,
		);
	}
}

use character_rigs::RigPose;

fn apply_jump_transition<A, R>(
	cache: Option<&mut JumpTransitionCache>,
	key: JumpTransitionKey,
	rig: &mut R,
	uncached_from_pose: impl FnOnce(&mut R) -> RigPose,
	capture_masked: impl FnOnce(&mut R) -> MaskedTransitionSource,
	transition: &Transition<A, R>,
	animation_progress: f32,
	transition_progress: f32,
) -> Effects
where
	A: Animation<R>,
	R: HumanoidRig,
{
	let from_pose = match cache {
		Some(cache) => {
			let source = cache.get_or_capture(key, rig, capture_masked);
			source.merge_into_current(rig)
		}
		None => uncached_from_pose(rig),
	};
	apply_with_from_pose(transition, rig, &from_pose, animation_progress, transition_progress)
}

#[cfg(test)]
mod tests {
	use std::hint::black_box;
	use std::time::Instant;

	use character_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::{Squat, DEFAULT_SPRING_DURATION};
	use crate::rigs::mix::seed_bind_pose;

	fn default_jump() -> TwoFootedJump<HumanoidV0Rig> {
		TwoFootedJump::default()
	}

	fn assert_poses_match(expected: &HumanoidV0Rig, actual: &HumanoidV0Rig) {
		for bone in expected.animation_bones() {
			let e = expected.pose().get(&bone).expect("expected bone");
			let a = actual.pose().get(&bone).expect("actual bone");
			assert!(
				(e.swing - a.swing).abs() < 1e-5
					&& (e.flex - a.flex).abs() < 1e-5
					&& (e.twist - a.twist).abs() < 1e-5,
				"bone {:?} mismatch: expected swing={} flex={} twist={}, got swing={} flex={} twist={}",
				bone,
				e.swing,
				e.flex,
				e.twist,
				a.swing,
				a.flex,
				a.twist,
			);
		}
	}

	fn sample_jump_uncached(
		jump: &TwoFootedJump<HumanoidV0Rig>,
		rig: &mut HumanoidV0Rig,
		elapsed: f32,
	) {
		jump.apply_for(rig, elapsed);
	}

	fn sample_jump_cached(
		jump: &TwoFootedJump<HumanoidV0Rig>,
		rig: &mut HumanoidV0Rig,
		elapsed: f32,
		cache: &mut JumpTransitionCache,
	) {
		jump.apply_for_with_cache(rig, elapsed, cache);
	}

	#[test]
	fn spring_end_legs_straight() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths();
		let elapsed = jump.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.99;
		jump.apply(&mut rig, elapsed);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(femur.swing.abs() < 0.05);
		assert!(shin.flex.abs() < 0.05);
		Ok(())
	}

	#[test]
	fn land_starts_compression_after_touchdown() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		crate::rigs::mix::seed_bind_pose(&mut rig);
		let jump = default_jump();
		let lengths = rig.segment_lengths();
		let timings = jump.timings(lengths);
		jump.apply(&mut rig, timings.air_end() + timings.land_descent_duration * 0.25);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		assert!(femur.swing.abs() > 0.01);
		Ok(())
	}

	#[test]
	fn land_peak_below_full_squat() -> anyhow::Result<()> {
		let mut rig_squat = HumanoidV0Rig::imported();
		Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0).apply(&mut rig_squat, 0.5);
		let squat_femur = rig_squat
			.pose()
			.get(&rig_squat.leg(Side::Left).femur.name)
			.expect("femur")
			.swing;

		let mut rig_land = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig_land.segment_lengths();
		let timings = jump.timings(lengths);
		jump.apply(&mut rig_land, timings.air_end() + timings.land_descent_duration * 0.99);
		let land_femur =
			rig_land.pose().get(&rig_land.leg(Side::Left).femur.name).expect("femur").swing;

		assert!(land_femur.abs() < squat_femur.abs());
		Ok(())
	}

	#[test]
	fn windup_still_drops_the_armature() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths();
		let mid_windup = jump.timings(lengths).squat_descent_duration * 0.99;
		let effects = jump.apply(&mut rig, mid_windup);
		let Some(tf) = effects.r#move else {
			return Err(anyhow::anyhow!("jump windup must keep Effects.move"));
		};
		if tf.translation.y >= 0.0 {
			return Err(anyhow::anyhow!(
				"windup drop should be negative Y, got {}",
				tf.translation.y
			));
		}
		Ok(())
	}

	#[test]
	fn land_transition_blends_arms_from_fall() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		crate::rigs::mix::seed_bind_pose(&mut rig);
		let jump = default_jump();
		let lengths = rig.segment_lengths();
		let timings = jump.timings(lengths);
		let blend = timings.land_pose_blend_duration();
		jump.apply(&mut rig, timings.air_end() + blend * 0.5);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		let fall_shoulder = Fall::<HumanoidV0Rig>::default().shoulder_flex(Side::Left, 1.0);
		assert!(shoulder.flex.abs() > 0.05);
		assert!(shoulder.flex.abs() < fall_shoulder.abs());
		Ok(())
	}

	#[test]
	fn cached_jump_matches_uncached_across_segments() -> anyhow::Result<()> {
		let jump = default_jump();
		let lengths = HumanoidV0Rig::imported().segment_lengths();
		let timings = jump.timings(lengths);
		let checkpoints = [
			timings.squat_descent_duration * 0.5,
			timings.squat_end() - 0.01,
			timings.squat_end() + timings.spring_duration * 0.25,
			timings.squat_end() + timings.spring_duration * 0.75,
			timings.spring_end() + timings.air_duration * 0.1,
			timings.spring_end() + FALL_BLEND_FRACTION * timings.air_duration * 0.5,
			timings.air_end() + timings.land_pose_blend_duration() * 0.5,
			timings.air_end() + timings.land_descent_duration * 0.5,
			timings.cycle_duration() - 0.05,
		];

		let mut uncached = HumanoidV0Rig::imported();
		let mut cached = HumanoidV0Rig::imported();
		seed_bind_pose(&mut uncached);
		seed_bind_pose(&mut cached);
		let mut cache = JumpTransitionCache::default();
		let mut elapsed = 0.0_f32;
		let mut checkpoint = 0_usize;

		while checkpoint < checkpoints.len() {
			let target = checkpoints[checkpoint];
			while elapsed + 1e-4 < target {
				elapsed = (elapsed + 0.01).min(target);
				sample_jump_uncached(&jump, &mut uncached, elapsed);
				sample_jump_cached(&jump, &mut cached, elapsed, &mut cache);
			}
			assert_poses_match(&uncached, &cached);
			checkpoint += 1;
		}
		Ok(())
	}

	#[test]
	fn cached_jump_matches_uncached_after_restart() -> anyhow::Result<()> {
		let jump = default_jump();
		let lengths = HumanoidV0Rig::imported().segment_lengths();
		let timings = jump.timings(lengths);
		let mid_air = timings.spring_end() + timings.air_duration * 0.5;

		let mut uncached = HumanoidV0Rig::imported();
		seed_bind_pose(&mut uncached);
		sample_jump_uncached(&jump, &mut uncached, mid_air);
		sample_jump_uncached(&jump, &mut uncached, timings.squat_end() + 0.05);

		let mut cached = HumanoidV0Rig::imported();
		seed_bind_pose(&mut cached);
		let mut cache = JumpTransitionCache::default();
		sample_jump_cached(&jump, &mut cached, mid_air, &mut cache);
		sample_jump_cached(&jump, &mut cached, timings.squat_end() + 0.05, &mut cache);

		assert_poses_match(&uncached, &cached);
		Ok(())
	}

	#[test]
	fn cached_jump_preserves_unrelated_bone_motion_during_spring() -> anyhow::Result<()> {
		let jump = default_jump();
		let lengths = HumanoidV0Rig::imported().segment_lengths();
		let timings = jump.timings(lengths);
		let spring_start = timings.squat_end();
		let spring_end = spring_start + timings.spring_duration * 0.4;
		let shoulder_name = HumanoidV0Rig::imported().arm(Side::Left).shoulder.name.clone();

		let mut uncached = HumanoidV0Rig::imported();
		let mut cached = HumanoidV0Rig::imported();
		seed_bind_pose(&mut uncached);
		seed_bind_pose(&mut cached);
		let mut cache = JumpTransitionCache::default();

		let mut elapsed = 0.0_f32;
		while elapsed + 1e-4 < spring_start {
			elapsed = (elapsed + 0.01).min(spring_start);
			sample_jump_uncached(&jump, &mut uncached, elapsed);
			sample_jump_cached(&jump, &mut cached, elapsed, &mut cache);
		}

		for rig in [&mut uncached, &mut cached] {
			let shoulder = rig.pose().get(&shoulder_name).expect("shoulder").clone();
			rig.pose_mut().insert(character_rigs::BonePose {
				name: shoulder.name.clone(),
				transform: shoulder.transform,
				swing: shoulder.swing + 0.2,
				flex: shoulder.flex + 0.15,
				twist: shoulder.twist,
			});
		}

		while elapsed + 1e-4 < spring_end {
			elapsed = (elapsed + 0.01).min(spring_end);
			sample_jump_uncached(&jump, &mut uncached, elapsed);
			sample_jump_cached(&jump, &mut cached, elapsed, &mut cache);
		}

		assert_poses_match(&uncached, &cached);
		Ok(())
	}

	#[test]
	#[ignore = "microbench: run with --release -- --ignored"]
	fn bench_jump_transition_capture() -> anyhow::Result<()> {
		let jump = default_jump();
		let lengths = HumanoidV0Rig::imported().segment_lengths();
		let timings = jump.timings(lengths);
		let spring_elapsed = timings.squat_end() + timings.spring_duration * 0.5;
		const RUNS: u32 = 200;
		const WARMUP: u32 = 20;

		fn median(samples: &mut [std::time::Duration]) -> std::time::Duration {
			samples.sort();
			samples[samples.len() / 2]
		}

		let mut uncached_rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut uncached_rig);
		for _ in 0..WARMUP {
			sample_jump_uncached(&jump, &mut uncached_rig, spring_elapsed);
		}
		let mut uncached_samples = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			black_box(sample_jump_uncached(&jump, &mut uncached_rig, spring_elapsed));
			uncached_samples.push(start.elapsed());
		}
		let min_uncached = *uncached_samples.iter().min().expect("samples");
		let median_uncached = median(&mut uncached_samples);

		let mut cached_rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut cached_rig);
		let mut cache = JumpTransitionCache::default();
		for _ in 0..WARMUP {
			cache.invalidate();
			sample_jump_cached(&jump, &mut cached_rig, spring_elapsed, &mut cache);
		}
		let mut cached_first_samples = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			cache.invalidate();
			let start = Instant::now();
			black_box(sample_jump_cached(&jump, &mut cached_rig, spring_elapsed, &mut cache));
			cached_first_samples.push(start.elapsed());
		}
		let min_cached_first = *cached_first_samples.iter().min().expect("samples");
		let median_cached_first = median(&mut cached_first_samples);

		let mut cached_hit_samples = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			black_box(sample_jump_cached(&jump, &mut cached_rig, spring_elapsed, &mut cache));
			cached_hit_samples.push(start.elapsed());
		}
		let min_cached_hit = *cached_hit_samples.iter().min().expect("samples");
		let median_cached_hit = median(&mut cached_hit_samples);

		eprintln!(
			"jump spring segment (1 HumanoidV0Rig, TwoFootedJump::apply_for, {} runs):",
			RUNS
		);
		eprintln!(
			"  uncached (capture every frame): min={:?} median={:?}",
			min_uncached,
			median_uncached,
		);
		eprintln!(
			"  cached first frame (segment entry capture): min={:?} median={:?}",
			min_cached_first,
			median_cached_first,
		);
		eprintln!(
			"  cached hit (reuse masked source): min={:?} median={:?}",
			min_cached_hit,
			median_cached_hit,
		);
		Ok(())
	}
}
