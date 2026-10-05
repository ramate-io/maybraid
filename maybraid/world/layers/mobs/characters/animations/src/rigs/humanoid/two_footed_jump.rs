use bevy::prelude::{Transform, Vec3};
use character_rigs::{humanoid::HumanoidRig, Side};
use log::info;

use crate::animations::{
	Fall, JumpSegment, Spring, Squat, Transition, TransitionCurve, TwoFootedJump,
};
use crate::rigs::transition::capture_animation_pose;
use crate::{Animation, Effects};

fn segment_debug_enabled() -> bool {
	std::env::var("CROZON_ANIMATION_DEBUG").is_ok()
}

impl<R: HumanoidRig> Animation<R> for TwoFootedJump<R> {
	fn apply_for(&self, rig: &mut R, elapsed: f32) {
		let lengths = rig.segment_lengths();
		let (derived, sample) = self.sample(lengths, elapsed);
		let timings = derived.timings;
		let local = sample.local_progress;

		match sample.segment {
			JumpSegment::Squat => {
				let progress = local / timings.squat_duration().max(f32::EPSILON);
				derived.prejump_squat::<R>().apply_for(rig, progress);
			}
			JumpSegment::Spring => {
				let from_pose = capture_animation_pose(&Squat::<R>::for_loop(1.0, 1.0), rig, 0.0);
				let _ = Transition::from_pose(Spring::<R>::default(), from_pose)
					.with_curve(TransitionCurve::SmoothStep)
					.apply(rig, local, local);
			}
			JumpSegment::Fall => {
				let fall = Fall::<R>::default();
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
				if sample.transition_weight < 1.0 {
					let from_pose = capture_animation_pose(&Spring::<R>::default(), rig, 1.0);
					let _ = Transition::from_pose(fall, from_pose)
						.with_curve(TransitionCurve::SmoothStep)
						.apply(rig, local, sample.transition_weight);
				} else {
					fall.apply_for(rig, local);
				}
			}
			JumpSegment::Land => {
				let land = derived.landing_squat::<R>();
				let land_duration = timings.land_duration().max(f32::EPSILON);
				let land_progress = local / land_duration;
				if segment_debug_enabled() && local < timings.land_descent_duration + 0.05 {
					info!(
						"jump land start: elapsed={:.3} cycle_t={:.3} land_local={:.4} land_depth={:.4} land_femur={:.4} transition={:.4} land_desc_d={:.4} y={:.4}",
						elapsed,
						self.time_in_cycle(lengths, elapsed),
						local,
						land.depth(land_progress),
						land.femur_swing(land_progress),
						sample.transition_weight,
						timings.land_descent_duration,
						sample.vertical_offset,
					);
				}
				if sample.transition_weight < 1.0 {
					let from_pose = capture_animation_pose(&Fall::<R>::default(), rig, 1.0);
					let _ = Transition::from_pose(land, from_pose)
						.with_curve(TransitionCurve::SmoothStep)
						.apply(rig, land_progress, sample.transition_weight);
				} else {
					land.apply_for(rig, land_progress);
				}
			}
		}
	}

	fn effects_for(&self, rig: &R, elapsed: f32) -> Effects {
		let lengths = rig.segment_lengths();
		let derived = self.rig_derived(lengths);
		let sample = self.cached_sample(lengths, elapsed, &derived);
		Effects {
			r#move: (sample.vertical_offset.abs() > f32::EPSILON)
				.then(|| Transform::from_translation(Vec3::new(0.0, sample.vertical_offset, 0.0))),
		}
	}
}

impl<R: HumanoidRig> TwoFootedJump<R> {
	pub fn log_landing_debug(&self, rig: &R, elapsed: f32, label: &str) {
		let lengths = rig.segment_lengths();
		let derived = self.rig_derived(lengths);
		let sample = self.cached_sample(lengths, elapsed, &derived);
		let timings = derived.timings;
		let land_progress = sample.local_progress / timings.land_duration().max(f32::EPSILON);

		info!(
			"{label}: elapsed={:.3} cycle_t={:.3} segment={:?} local={:.4} land_depth={:.4} y={:.4} timings[squat=({:.3},{:.3}) spring={:.3} air={:.3} land=({:.4},{:.3})] speeds[pre={:.3} landing={:.3}]",
			elapsed,
			self.time_in_cycle(lengths, elapsed),
			sample.segment,
			sample.local_progress,
			derived.landing_squat::<R>().depth(land_progress),
			sample.vertical_offset,
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

#[cfg(test)]
mod tests {
	use std::hint::black_box;
	use std::time::Instant;

	use character_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::{Squat, DEFAULT_SPRING_DURATION};

	fn default_jump() -> TwoFootedJump<HumanoidV0Rig> {
		TwoFootedJump::default()
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
	fn split_apply_and_effects_share_cached_sample() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let elapsed = 1.37;
		jump.apply_for(&mut rig, elapsed);
		let effects = jump.effects_for(&rig, elapsed);
		let lengths = rig.segment_lengths();
		let derived = jump.rig_derived(lengths);
		let sample = jump.cached_sample(lengths, elapsed, &derived);
		let y = sample.vertical_offset;
		assert_eq!(effects.r#move.map(|t| t.translation.y), (y.abs() > f32::EPSILON).then_some(y),);
		Ok(())
	}

	mod legacy {
		use character_rigs::humanoid::{HumanoidRig, LegSegmentLengths};

		use crate::animations::{
			Fall, JumpSegment, JumpTiming, Land, Squat, TwoFootedJump, DEFAULT_SPRING_DURATION,
			FALL_BLEND_FRACTION,
		};

		const MIN_SEGMENT_DURATION: f32 = 1e-3;
		const MIN_SPEED: f32 = 1e-3;
		use crate::{Animation, Effects};

		fn squat_configs<Rig>(
			jump: &TwoFootedJump<Rig>,
			lengths: LegSegmentLengths,
		) -> (Squat<Rig>, Land<Rig>) {
			let impact = crate::animations::launch_speed(jump.gravity, jump.jump_height);
			let squat_peak = Squat::<Rig>::default().peak_vertical_drop(lengths);
			let windup_descent = jump.pre_squat_speed.max(MIN_SPEED);
			let windup_ascent =
				if squat_peak > f32::EPSILON { impact / squat_peak } else { windup_descent };
			let land_half_speed = jump.landing_squat_speed.max(MIN_SPEED);
			let windup = Squat::with_speeds(windup_descent, windup_ascent.max(MIN_SPEED));
			let landing =
				Land::with_speeds(land_half_speed, land_half_speed, Squat::<Rig>::default());
			(windup, landing)
		}

		fn timings<Rig>(jump: &TwoFootedJump<Rig>, lengths: LegSegmentLengths) -> JumpTiming {
			let (windup, landing) = squat_configs(jump, lengths);
			let touchdown =
				crate::animations::touchdown_time_since_launch(jump.gravity, jump.jump_height);
			let fall_duration = (touchdown - DEFAULT_SPRING_DURATION).max(MIN_SEGMENT_DURATION);
			JumpTiming {
				squat_descent_duration: windup.descent_duration().max(MIN_SEGMENT_DURATION),
				squat_ascent_duration: windup.ascent_duration().max(MIN_SEGMENT_DURATION),
				spring_duration: DEFAULT_SPRING_DURATION,
				air_duration: fall_duration,
				land_descent_duration: landing.descent_duration().max(MIN_SEGMENT_DURATION),
				land_ascent_duration: landing.ascent_duration().max(MIN_SEGMENT_DURATION),
			}
		}

		fn segment<Rig>(
			jump: &TwoFootedJump<Rig>,
			lengths: LegSegmentLengths,
			elapsed: f32,
		) -> (JumpSegment, f32) {
			let timings = timings(jump, lengths);
			let cycle = timings.cycle_duration();
			let time_in_cycle = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
			TwoFootedJump::<Rig>::segment_at_time(time_in_cycle, &timings)
		}

		fn vertical_offset<Rig>(
			jump: &TwoFootedJump<Rig>,
			lengths: LegSegmentLengths,
			elapsed: f32,
		) -> f32 {
			let (segment, local) = segment(jump, lengths, elapsed);
			let timings = timings(jump, lengths);
			let (prejump_squat, landing_squat) = squat_configs(jump, lengths);
			match segment {
				JumpSegment::Squat => {
					let p = local / timings.squat_duration().max(f32::EPSILON);
					-prejump_squat.vertical_drop(p, lengths)
				}
				JumpSegment::Spring | JumpSegment::Fall => {
					let cycle = timings.cycle_duration();
					let time_in_cycle = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
					let since_launch = (time_in_cycle - timings.squat_end()).max(0.0);
					jump.ballistic_height(since_launch.min(jump.touchdown_time_since_launch()))
				}
				JumpSegment::Land => {
					let p = local / timings.land_duration().max(f32::EPSILON);
					-landing_squat.vertical_drop(p, lengths)
				}
			}
		}

		pub(super) fn apply_for_sampling<R: HumanoidRig>(
			jump: &TwoFootedJump<R>,
			rig: &R,
			elapsed: f32,
		) {
			let lengths = rig.segment_lengths();
			let (segment, local) = segment(jump, lengths, elapsed);
			let timings = timings(jump, lengths);
			std::hint::black_box((segment, local, timings));
		}

		pub(super) fn effects_for_sampling<R: HumanoidRig>(
			jump: &TwoFootedJump<R>,
			rig: &R,
			elapsed: f32,
		) {
			let lengths = rig.segment_lengths();
			let y = vertical_offset(jump, lengths, elapsed);
			std::hint::black_box(y);
		}

		pub(super) fn apply_for<R: HumanoidRig>(
			jump: &TwoFootedJump<R>,
			rig: &mut R,
			elapsed: f32,
		) {
			let lengths = rig.segment_lengths();
			let (segment, local) = segment(jump, lengths, elapsed);
			let timings = timings(jump, lengths);

			match segment {
				JumpSegment::Squat => {
					let squat = squat_configs(jump, lengths).0;
					let progress = local / timings.squat_duration().max(f32::EPSILON);
					squat.apply_for(rig, progress);
				}
				JumpSegment::Spring => {
					let from_pose = crate::rigs::transition::capture_animation_pose(
						&Squat::<R>::for_loop(1.0, 1.0),
						rig,
						0.0,
					);
					let _ = crate::animations::Transition::from_pose(
						crate::animations::Spring::<R>::default(),
						from_pose,
					)
					.with_curve(crate::animations::TransitionCurve::SmoothStep)
					.apply(rig, local, local);
				}
				JumpSegment::Fall => {
					let fall = Fall::<R>::default();
					let blend_end = FALL_BLEND_FRACTION;
					if local < blend_end {
						let from_pose = crate::rigs::transition::capture_animation_pose(
							&crate::animations::Spring::<R>::default(),
							rig,
							1.0,
						);
						let transition_progress = (local / blend_end).clamp(0.0, 1.0);
						let _ = crate::animations::Transition::from_pose(fall, from_pose)
							.with_curve(crate::animations::TransitionCurve::SmoothStep)
							.apply(rig, local, transition_progress);
					} else {
						fall.apply_for(rig, local);
					}
				}
				JumpSegment::Land => {
					let land = squat_configs(jump, lengths).1;
					let land_duration = timings.land_duration().max(f32::EPSILON);
					let land_progress = local / land_duration;
					let blend_window = timings.land_pose_blend_duration();
					let transition_progress = if blend_window > f32::EPSILON {
						(local / blend_window).clamp(0.0, 1.0)
					} else {
						1.0
					};
					if transition_progress < 1.0 {
						let from_pose = crate::rigs::transition::capture_animation_pose(
							&Fall::<R>::default(),
							rig,
							1.0,
						);
						let _ = crate::animations::Transition::from_pose(land, from_pose)
							.with_curve(crate::animations::TransitionCurve::SmoothStep)
							.apply(rig, land_progress, transition_progress);
					} else {
						land.apply_for(rig, land_progress);
					}
				}
			}
		}

		pub(super) fn effects_for<R: HumanoidRig>(
			jump: &TwoFootedJump<R>,
			rig: &R,
			elapsed: f32,
		) -> Effects {
			let lengths = rig.segment_lengths();
			let y = vertical_offset(jump, lengths, elapsed);
			Effects {
				r#move: (y.abs() > f32::EPSILON).then(|| {
					bevy::prelude::Transform::from_translation(bevy::prelude::Vec3::new(
						0.0, y, 0.0,
					))
				}),
			}
		}
	}

	fn mailbox_jump() -> TwoFootedJump<HumanoidV0Rig> {
		use crate::animations::{
			DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT, DEFAULT_LANDING_SQUAT_SPEED,
			DEFAULT_PRE_SQUAT_SPEED,
		};
		TwoFootedJump::default()
			.with_gravity(DEFAULT_GRAVITY)
			.with_jump_height(DEFAULT_JUMP_HEIGHT)
			.with_pre_squat_speed(DEFAULT_PRE_SQUAT_SPEED * 1.2)
			.with_landing_squat_speed(DEFAULT_LANDING_SQUAT_SPEED * 1.3)
	}

	fn bench_runs<F>(runs: usize, mut f: F) -> (std::time::Duration, std::time::Duration)
	where
		F: FnMut(),
	{
		let mut durations: Vec<std::time::Duration> = Vec::with_capacity(runs);
		for _ in 0..runs {
			let start = Instant::now();
			f();
			durations.push(start.elapsed());
		}
		durations.sort();
		(durations[0], durations[durations.len() / 2])
	}

	fn report_pair(
		label: &str,
		characters: usize,
		frames: usize,
		runs: usize,
		legacy: (std::time::Duration, std::time::Duration),
		optimized: (std::time::Duration, std::time::Duration),
	) {
		let legacy_median = legacy.1.as_secs_f64() * 1000.0;
		let optimized_median = optimized.1.as_secs_f64() * 1000.0;
		let delta_pct = if legacy_median > 0.0 {
			(legacy_median - optimized_median) / legacy_median * 100.0
		} else {
			0.0
		};
		eprintln!(
			"{label}: characters={characters} frames={frames} runs={runs} \
legacy[min={legacy_min:.3}ms median={legacy_med:.3}ms] \
optimized[min={opt_min:.3}ms median={opt_med:.3}ms] \
median_delta={delta:+.1}%",
			legacy_min = legacy.0.as_secs_f64() * 1000.0,
			legacy_med = legacy_median,
			opt_min = optimized.0.as_secs_f64() * 1000.0,
			opt_med = optimized_median,
			delta = delta_pct,
		);
	}

	#[test]
	#[ignore = "microbench: cargo test -p character-animations two_footed_jump_apply_split --release -- --ignored --nocapture"]
	fn two_footed_jump_apply_split_microbench() {
		use std::time::Duration;

		const CHARACTERS: usize = 64;
		const FRAMES: usize = 600;
		const RUNS: usize = 5;

		let jump = default_jump();
		let mut rigs: Vec<HumanoidV0Rig> =
			(0..CHARACTERS).map(|_| HumanoidV0Rig::imported()).collect();
		let elapsed_samples: Vec<f32> =
			(0..FRAMES).map(|i| i as f32 * 0.016 + (i % 17) as f32 * 0.003).collect();

		let legacy_sampling_reused = bench_runs(RUNS, || {
			for elapsed in &elapsed_samples {
				for rig in &rigs {
					legacy::apply_for_sampling(&jump, rig, black_box(*elapsed));
					legacy::effects_for_sampling(&jump, rig, black_box(*elapsed));
				}
			}
		});
		let optimized_sampling_reused = bench_runs(RUNS, || {
			for elapsed in &elapsed_samples {
				for rig in &rigs {
					let lengths = rig.segment_lengths();
					let derived = jump.rig_derived(lengths);
					let sample = jump.cached_sample(lengths, black_box(*elapsed), &derived);
					black_box(sample);
					black_box(jump.cached_sample(lengths, black_box(*elapsed), &derived));
				}
			}
		});
		report_pair(
			"split_sampling_reused",
			CHARACTERS,
			FRAMES,
			RUNS,
			legacy_sampling_reused,
			optimized_sampling_reused,
		);

		let legacy_sampling_mailbox = bench_runs(RUNS, || {
			for elapsed in &elapsed_samples {
				for rig in &rigs {
					let jump = mailbox_jump();
					legacy::apply_for_sampling(&jump, rig, black_box(*elapsed));
					legacy::effects_for_sampling(&jump, rig, black_box(*elapsed));
				}
			}
		});
		let optimized_sampling_mailbox = bench_runs(RUNS, || {
			for elapsed in &elapsed_samples {
				for rig in &rigs {
					let jump = mailbox_jump();
					let lengths = rig.segment_lengths();
					let derived = jump.rig_derived(lengths);
					let sample = jump.cached_sample(lengths, black_box(*elapsed), &derived);
					black_box(sample);
					black_box(jump.cached_sample(lengths, black_box(*elapsed), &derived));
				}
			}
		});
		report_pair(
			"split_sampling_mailbox",
			CHARACTERS,
			FRAMES,
			RUNS,
			legacy_sampling_mailbox,
			optimized_sampling_mailbox,
		);

		let legacy_full_reused = bench_runs(RUNS, || {
			for elapsed in &elapsed_samples {
				for rig in &mut rigs {
					legacy::apply_for(&jump, rig, black_box(*elapsed));
					black_box(legacy::effects_for(&jump, rig, black_box(*elapsed)));
				}
			}
		});
		let optimized_full_reused = bench_runs(RUNS, || {
			for elapsed in &elapsed_samples {
				for rig in &mut rigs {
					jump.apply_for(rig, black_box(*elapsed));
					black_box(jump.effects_for(rig, black_box(*elapsed)));
				}
			}
		});
		report_pair(
			"split_full_reused",
			CHARACTERS,
			FRAMES,
			RUNS,
			legacy_full_reused,
			optimized_full_reused,
		);

		let legacy_full_mailbox = bench_runs(RUNS, || {
			for elapsed in &elapsed_samples {
				for rig in &mut rigs {
					let jump = mailbox_jump();
					legacy::apply_for(&jump, rig, black_box(*elapsed));
					black_box(legacy::effects_for(&jump, rig, black_box(*elapsed)));
				}
			}
		});
		let optimized_full_mailbox = bench_runs(RUNS, || {
			for elapsed in &elapsed_samples {
				for rig in &mut rigs {
					let jump = mailbox_jump();
					jump.apply_for(rig, black_box(*elapsed));
					black_box(jump.effects_for(rig, black_box(*elapsed)));
				}
			}
		});
		report_pair(
			"split_full_mailbox",
			CHARACTERS,
			FRAMES,
			RUNS,
			legacy_full_mailbox,
			optimized_full_mailbox,
		);

		let _ = Duration::ZERO;
	}
}
